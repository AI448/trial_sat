use std::hint::unreachable_unchecked;

use crate::finite_collections::indirect_heap;
use crate::finite_collections::Array;

use super::types::{Reason, VariableSize};

//#[repr(align(64))] // TODO 後で検証（単に 64 byte にするとどうなるのか・size を 32byte に切り詰めて align を 32byte にするとどうなるのか）
// 32byte を超えてしまうようならメンバは構造体にしたほうがいいかも(先頭で 3 byte 無駄になるが 64 byte を超えない限り問題ない)
// ↑何が嬉しいんだっけ？
pub enum VariableState {
    Assigned { assigned_value: bool, decision_level: VariableSize, assignment_level: VariableSize, reason: Reason },
    Conflicting { last_assigned_value: bool, reasons: [Reason; 2] },
    TentativelyAssigned { last_assigned_value: bool, tentatively_assigned_value: bool, reason: Reason },
    Unassigned { last_assigned_value: bool },
}

impl VariableState {
    pub fn is_assigned(&self) -> bool {
        if let VariableState::Assigned { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_value_assigned(&self, value: bool) -> bool {
        if let VariableState::Assigned { assigned_value, .. } = self {
            *assigned_value == value
        } else {
            false
        }
    }
}

type ConflictingVariableScore = (f64, VariableSize, VariableSize);

type TentativelyAssigedVariableScore = (f64, VariableSize, VariableSize);

type UnassignedVariableScore = f64;

/// 変数の割り当て状態を管理する
pub struct Variables {
    decision_level: VariableSize,
    variable_states: Array<VariableSize, VariableState>,
    positions: Array<VariableSize, VariableSize>,
    assigned_variables: Array<VariableSize, VariableSize>,
    conflicting_variables: Array<VariableSize, (VariableSize, ConflictingVariableScore)>,
    tentatively_assigned_variables: Array<VariableSize, (VariableSize, TentativelyAssigedVariableScore)>,
    unassigned_variables: Array<VariableSize, (VariableSize, UnassignedVariableScore)>,
    activity_time_constant: f64,
    activity_increase_value: f64,
    activities: Array<VariableSize, f64>,
}

impl Variables {
    const NULL_POSITION: VariableSize = VariableSize::MAX;

    #[inline(never)]
    pub fn new(activity_time_constant: f64) -> Self {
        assert!(activity_time_constant.is_finite());
        assert!(activity_time_constant > 0.0);
        Variables {
            decision_level: 0,
            variable_states: Array::default(),
            positions: Array::default(),
            assigned_variables: Array::default(),
            conflicting_variables: Array::default(),
            tentatively_assigned_variables: Array::default(),
            unassigned_variables: Array::default(),
            activity_time_constant: activity_time_constant,
            activity_increase_value: 1.0,
            activities: Array::default(),
        }
    }

    #[inline(always)]
    pub fn dimension(&self) -> VariableSize {
        self.variable_states.len()
    }

    #[inline(always)]
    pub fn number_of_assigned_variables(&self) -> VariableSize {
        self.assigned_variables.len()
    }

    #[inline(always)]
    pub fn number_of_conflicting_variables(&self) -> VariableSize {
        self.conflicting_variables.len()
    }

    #[inline(always)]
    pub fn number_of_tentative_assigned_variables(&self) -> VariableSize {
        self.tentatively_assigned_variables.len()
    }

    #[inline(always)]
    pub fn number_of_unassigned_variables(&self) -> VariableSize {
        self.unassigned_variables.len()
    }

    #[inline(always)]
    pub fn current_decision_level(&self) -> VariableSize {
        self.decision_level
    }

    #[inline(always)]
    pub fn current_assignment_level(&self) -> VariableSize {
        self.assigned_variables.len()
    }

    #[inline(never)]
    pub fn redimension(&mut self, new_dimension: VariableSize) {
        assert!(new_dimension >= self.variable_states.len());
        while new_dimension > self.variable_states.len() {
            let index = self.variable_states.len();
            let initial_value = false;
            let initial_activity = 0.0;
            let priority = Self::calculate_unassigned_variable_score(initial_activity);
            // 未割り当て変数として初期化
            self.variable_states.push(VariableState::Unassigned { last_assigned_value: initial_value });
            self.activities.push(initial_activity);
            self.positions.push(Self::NULL_POSITION);
            // 未割り当て変数のキューに追加
            Self::push_heap_item(&mut self.positions, &mut self.unassigned_variables, index, priority);
            // 整合性チェック
            debug_assert!(self.variable_states.len() == self.activities.len());
            debug_assert!(self.variable_states.len() == self.positions.len());
        }
    }

    pub fn first_unassigned_variable(&self) -> Option<(VariableSize, &VariableState)> {
        match self.unassigned_variables.first() {
            Some((index, ..)) => {
                debug_assert!(matches!(self.variable_states[*index], VariableState::Unassigned { .. }));
                Some((*index, &self.variable_states[*index]))
            }
            None => None,
        }
    }

    pub fn first_tentatively_assigned_variable(&self) -> Option<(VariableSize, &VariableState)> {
        match self.tentatively_assigned_variables.first() {
            Some((index, ..)) => {
                debug_assert!(matches!(self.variable_states[*index], VariableState::TentativelyAssigned { .. }));
                Some((*index, &self.variable_states[*index]))
            }
            None => None,
        }
    }

    pub fn first_conflicting_variable(&self) -> Option<(VariableSize, &VariableState)> {
        match self.conflicting_variables.first() {
            Some((index, ..)) => {
                debug_assert!(matches!(self.variable_states[*index], VariableState::Conflicting { .. }));
                Some((*index, &self.variable_states[*index]))
            }
            None => None,
        }
    }

    #[inline(always)]
    pub fn get(&self, index: VariableSize) -> &VariableState {
        &self.variable_states[index]
    }

    #[inline(always)]
    pub fn tentatively_assign(&mut self, index: VariableSize, value: bool, reason: Reason) {
        let variable_state = &mut self.variable_states[index];
        // 現在の割当状態に応じて場合分け
        match variable_state {
            VariableState::Unassigned { last_assigned_value } => {
                // 未割り当ての場合
                // 仮割当状態に遷移
                let score = Self::calculate_tentatively_assigned_variable_score(&reason, self.activities[index]);
                *variable_state = VariableState::TentativelyAssigned {
                    last_assigned_value: *last_assigned_value,
                    tentatively_assigned_value: value,
                    reason: reason,
                };
                Self::pop_heap_item(&mut self.positions, &mut self.unassigned_variables, index);
                Self::push_heap_item(&mut self.positions, &mut self.tentatively_assigned_variables, index, score);
            }
            VariableState::TentativelyAssigned {
                last_assigned_value,
                tentatively_assigned_value,
                reason: original_reason,
                ..
            } => {
                if value == *tentatively_assigned_value {
                    // 同じ値に仮割当されてる場合
                    let new_score =
                        Self::calculate_tentatively_assigned_variable_score(&reason, self.activities[index]);
                    let current_score = self.tentatively_assigned_variables[self.positions[index]].1;
                    if new_score < current_score {
                        // priority を改善するなら上書き
                        *original_reason = reason;
                        Self::change_heap_value(
                            &mut self.positions,
                            &mut self.tentatively_assigned_variables,
                            index,
                            new_score,
                        )
                    }
                } else {
                    // 異なる値に仮割当されてる場合
                    // Conflicting 状態に遷移
                    let reasons = if value == false { [reason, *original_reason] } else { [*original_reason, reason] };
                    let score = Self::calculate_conflicting_variable_score(&reasons, self.activities[index]);
                    *variable_state =
                        VariableState::Conflicting { last_assigned_value: *last_assigned_value, reasons: reasons };
                    Self::pop_heap_item(&mut self.positions, &mut self.tentatively_assigned_variables, index);
                    Self::push_heap_item(&mut self.positions, &mut self.conflicting_variables, index, score);
                }
            }
            VariableState::Conflicting { reasons: current_reasons, .. } => {
                // 既に矛盾している場合
                let new_reasons =
                    if value == false { [reason, current_reasons[1]] } else { [current_reasons[0], reason] };
                let new_score = Self::calculate_conflicting_variable_score(&new_reasons, self.activities[index]);
                let current_score = self.conflicting_variables[self.positions[index]].1;
                if new_score < current_score {
                    // score を改善するなら上書き
                    *current_reasons = new_reasons;
                    Self::change_heap_value(&mut self.positions, &mut self.conflicting_variables, index, new_score);
                }
            }
            VariableState::Assigned { .. } => {
                unreachable!();
            }
        }
    }

    #[inline(always)]
    pub fn assign(&mut self, index: VariableSize) {
        let variable_state = &mut self.variable_states[index];
        match variable_state {
            VariableState::TentativelyAssigned { tentatively_assigned_value, reason, .. } => {
                if let Reason::Decision = reason {
                    self.decision_level += 1;
                }
                let stack_position = self.assigned_variables.len();
                *variable_state = VariableState::Assigned {
                    assigned_value: *tentatively_assigned_value,
                    decision_level: self.decision_level,
                    assignment_level: stack_position + 1,
                    reason: *reason,
                };
                Self::pop_heap_item(&mut self.positions, &mut self.tentatively_assigned_variables, index);
                //
                debug_assert!(self.positions[index] == Self::NULL_POSITION);
                self.positions[index] = stack_position;
                self.assigned_variables.push(index);
            }
            VariableState::Assigned { .. } => {
                unreachable!();
            }
            VariableState::Conflicting { .. } => {
                unreachable!();
            }
            VariableState::Unassigned { .. } => {
                unreachable!();
            }
        }
    }

    #[inline(never)]
    pub fn cancel_tentative_assignment(&mut self) {
        while !self.conflicting_variables.is_empty() {
            let index = unsafe { self.conflicting_variables.first().unwrap_unchecked().0 };
            let variable_state = &mut self.variable_states[index];
            debug_assert!(matches!(variable_state, VariableState::Conflicting { .. }));
            let VariableState::Conflicting { last_assigned_value, .. } = variable_state else {
                unsafe { unreachable_unchecked() }
            };
            let priority = Self::calculate_unassigned_variable_score(self.activities[index]);
            *variable_state = VariableState::Unassigned { last_assigned_value: *last_assigned_value };
            Self::pop_heap_item(&mut self.positions, &mut self.conflicting_variables, index);
            Self::push_heap_item(&mut self.positions, &mut self.unassigned_variables, index, priority);
        }
        while !self.tentatively_assigned_variables.is_empty() {
            let index = unsafe { self.tentatively_assigned_variables.first().unwrap_unchecked().0 };
            let variable_state = &mut self.variable_states[index];
            debug_assert!(matches!(variable_state, VariableState::TentativelyAssigned { .. }));
            let VariableState::TentativelyAssigned { last_assigned_value, .. } = variable_state else {
                unsafe { unreachable_unchecked() }
            };
            let score = Self::calculate_unassigned_variable_score(self.activities[index]);
            *variable_state = VariableState::Unassigned { last_assigned_value: *last_assigned_value };
            Self::pop_heap_item(&mut self.positions, &mut self.tentatively_assigned_variables, index);
            Self::push_heap_item(&mut self.positions, &mut self.unassigned_variables, index, score);
        }
    }

    #[inline(always)]
    pub fn unassign(&mut self) -> VariableSize {
        assert!(!self.assigned_variables.is_empty());
        let index = unsafe { self.assigned_variables.pop().unwrap_unchecked() };
        debug_assert!(self.positions[index] == self.assigned_variables.len());
        self.positions[index] = Self::NULL_POSITION;
        let variable_state = &mut self.variable_states[index];
        debug_assert!(matches!(variable_state, VariableState::Assigned { .. }));
        let VariableState::Assigned { assigned_value, decision_level, assignment_level, reason } = variable_state
        else {
            unsafe {
                unreachable_unchecked();
            }
        };
        debug_assert!(*decision_level == self.decision_level);
        debug_assert!(*assignment_level == self.assigned_variables.len() + 1);
        if let Reason::Decision = reason {
            self.decision_level -= 1;
        }
        *variable_state = VariableState::Unassigned { last_assigned_value: *assigned_value };
        Self::push_heap_item(
            &mut self.positions,
            &mut self.unassigned_variables,
            index,
            Self::calculate_unassigned_variable_score(self.activities[index]),
        );
        index
    }

    #[inline(always)]
    pub fn increase_activity(&mut self, index: VariableSize) {
        self.activities[index] += self.activity_increase_value;
        if self.activities[index] > 1e4 {
            for activity in self.activities.iter_mut() {
                *activity /= self.activity_increase_value;
            }
            self.activity_increase_value = 1.0;
            // NOTE: 変更に際して注意が必要( activity を定数倍しても unassigned_variables をソートし直す必要がないという仮定を使っている)
            for (index, priority) in self.unassigned_variables.iter_mut() {
                *priority = Self::calculate_unassigned_variable_score(self.activities[*index]);
            }
        }
    }

    #[inline(always)]
    pub fn advance_time(&mut self) {
        self.activity_increase_value /= 1.0 - 1.0 / self.activity_time_constant;
    }

    #[inline(always)]
    fn calculate_conflicting_variable_score(reasons: &[Reason; 2], activity: f64) -> ConflictingVariableScore {
        debug_assert!(matches!(reasons[0], Reason::Propagation { .. }));
        debug_assert!(matches!(reasons[1], Reason::Propagation { .. }));
        let Reason::Propagation { lbd: lbd0, clause_length: clause_length0, .. } = reasons[0] else {
            unsafe {
                unreachable_unchecked();
            }
        };
        let Reason::Propagation { lbd: lbd1, clause_length: clause_length1, .. } = reasons[1] else {
            unsafe {
                unreachable_unchecked();
            }
        };
        (-activity, lbd0 + lbd1, clause_length0 + clause_length1)
    }

    #[inline(always)]
    fn calculate_tentatively_assigned_variable_score(
        reason: &Reason,
        activity: f64,
    ) -> TentativelyAssigedVariableScore {
        match reason {
            Reason::Decision => (-f64::INFINITY, 0, 0),
            Reason::Propagation { lbd, clause_length, .. } => (-activity, *lbd, *clause_length),
        }
    }

    #[inline(always)]
    fn calculate_unassigned_variable_score(activity: f64) -> UnassignedVariableScore {
        -activity
    }

    #[inline(always)]
    fn push_heap_item<T>(
        positions: &mut Array<VariableSize, VariableSize>,
        heap: &mut Array<VariableSize, (VariableSize, T)>,
        index: VariableSize,
        value: T,
    ) where
        T: std::cmp::PartialOrd,
    {
        debug_assert!(positions[index] == Self::NULL_POSITION);
        let position = heap.len();
        positions[index] = position;
        heap.push((index, value));
        Self::up_heap(positions, heap, position);
    }

    #[inline(always)]
    fn change_heap_value<T>(
        positions: &mut Array<VariableSize, VariableSize>,
        heap: &mut Array<VariableSize, (VariableSize, T)>,
        index: VariableSize,
        value: T,
    ) where
        T: std::cmp::PartialOrd,
    {
        let position = positions[index];
        debug_assert!(heap[position].0 == index);
        heap[position].1 = value;
        Self::update_heap(positions, heap, position);
    }

    fn pop_heap_item<T>(
        positions: &mut Array<VariableSize, VariableSize>,
        heap: &mut Array<VariableSize, (VariableSize, T)>,
        index: VariableSize,
    ) where
        T: std::cmp::PartialOrd,
    {
        let position = positions[index];
        debug_assert!(heap[position].0 == index);
        positions[index] = Self::NULL_POSITION;
        if position + 1 == heap.len() {
            heap.pop();
        } else {
            heap.swap_remove(position).0;
            positions[heap[position].0] = position;
            Self::update_heap(positions, heap, position);
        }
    }

    #[inline(always)]
    fn update_heap<T>(
        positions: &mut Array<VariableSize, VariableSize>,
        heap: &mut Array<VariableSize, (VariableSize, T)>,
        position: VariableSize,
    ) where
        T: std::cmp::PartialOrd,
    {
        indirect_heap::update_heap(heap, positions, position, &|lhs, rhs| lhs.1 < rhs.1);
    }

    #[inline(always)]
    fn down_heap<T>(
        positions: &mut Array<VariableSize, VariableSize>,
        heap: &mut Array<VariableSize, (VariableSize, T)>,
        position: VariableSize,
    ) where
        T: std::cmp::PartialOrd,
    {
        indirect_heap::down_heap(heap, positions, position, &|lhs, rhs| lhs.1 < rhs.1);
    }

    #[inline(always)]
    fn up_heap<T>(
        positions: &mut Array<VariableSize, VariableSize>,
        heap: &mut Array<VariableSize, (VariableSize, T)>,
        position: VariableSize,
    ) where
        T: std::cmp::PartialOrd,
    {
        indirect_heap::up_heap(heap, positions, position, &|lhs, rhs| lhs.1 < rhs.1);
    }
}
