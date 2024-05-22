use super::types::Literal;

/// 割り当て理由
#[derive(Clone, Copy)]
pub enum Reason {
    Decision,
    Propagation { clause_index: usize, assignment_level_at_propagated: usize },
}

/// 変数の状態
// NOTE: 廃止するかも
pub enum VariableState {
    Assigned { value: bool, decision_level: usize, assignment_level: usize, reason: Reason },
    Unassigned { last_assigned_value: bool },
}

/// 変数の割り当て状態を管理する
#[derive(Default)]
pub struct VariableManager {
    decision_level: usize,
    assignment_infos: Vec<AssignmentInfo>,
    variable_infos: Vec<VariableInfo>,
}

impl VariableManager {
    #[inline(always)]
    pub fn number_of_variables(&self) -> usize {
        self.variable_infos.len()
    }

    #[inline(always)]
    pub fn number_of_assigned_variables(&self) -> usize {
        self.assignment_infos.len()
    }

    #[inline(always)]
    pub fn number_of_unassigned_variables(&self) -> usize {
        self.variable_infos.len() - self.assignment_infos.len()
    }

    #[inline(always)]
    pub fn current_decision_level(&self) -> usize {
        self.decision_level
    }

    #[inline(always)]
    pub fn current_assignment_level(&self) -> usize {
        self.assignment_infos.len()
    }

    pub fn expand(&mut self, additional: usize) {
        self.variable_infos.resize_with(self.variable_infos.len() + additional, || VariableInfo {
            value: false,
            assignment_level: VariableInfo::NULL_ASSIGNMENT_LEVEL,
        });
    }

    #[inline(always)]
    pub fn is_assigned(&self, literal: Literal) -> bool {
        self.variable_infos[literal.index].assignment_level != VariableInfo::NULL_ASSIGNMENT_LEVEL
    }

    #[inline(always)]
    pub fn is_true(&self, literal: Literal) -> bool {
        self.variable_infos[literal.index].assignment_level != VariableInfo::NULL_ASSIGNMENT_LEVEL
            && self.variable_infos[literal.index].value == literal.sign
    }

    #[inline(always)]
    pub fn is_false(&self, literal: Literal) -> bool {
        self.variable_infos[literal.index].assignment_level != VariableInfo::NULL_ASSIGNMENT_LEVEL
            && self.variable_infos[literal.index].value == !literal.sign
    }

    #[inline(always)]
    pub fn get_state(&self, index: usize) -> VariableState {
        // TODO: 検討． is_* 系を実装するならこの関数は不要では？ get_assignment_info, get_last_assigned_value に分けてもいい気がする．
        let variable_info = &self.variable_infos[index];
        if variable_info.assignment_level == VariableInfo::NULL_ASSIGNMENT_LEVEL {
            VariableState::Unassigned { last_assigned_value: variable_info.value }
        } else {
            let assignment_info = &self.assignment_infos[variable_info.assignment_level - 1];
            VariableState::Assigned {
                value: variable_info.value,
                assignment_level: variable_info.assignment_level,
                decision_level: assignment_info.decision_level,
                reason: assignment_info.reason,
            }
        }
    }

    #[inline(always)]
    pub fn assign(&mut self, variable_index: usize, value: bool, reason: Reason) {
        let variable_info = &mut self.variable_infos[variable_index];
        assert!(variable_info.assignment_level == VariableInfo::NULL_ASSIGNMENT_LEVEL);
        let assignment_level = self.assignment_infos.len() + 1;
        if let Reason::Decision = reason {
            self.decision_level += 1;
        }
        self.assignment_infos.push(AssignmentInfo {
            variable_index,
            decision_level: self.decision_level,
            reason: reason,
        });
        variable_info.value = value;
        variable_info.assignment_level = assignment_level;
    }

    #[inline(always)]
    pub fn unassign(&mut self) -> usize {
        assert!(!self.assignment_infos.is_empty());
        let assignment_info = self.assignment_infos.pop().unwrap();
        let variable_info = &mut self.variable_infos[assignment_info.variable_index];
        assert!(variable_info.assignment_level == self.assignment_infos.len() + 1);
        assert!(assignment_info.decision_level == self.decision_level);
        if let Reason::Decision = assignment_info.reason {
            self.decision_level -= 1;
        }
        variable_info.assignment_level = VariableInfo::NULL_ASSIGNMENT_LEVEL;
        assignment_info.variable_index
    }
}

struct AssignmentInfo {
    pub variable_index: usize,
    pub decision_level: usize,
    pub reason: Reason,
}

struct VariableInfo {
    pub value: bool,
    pub assignment_level: usize,
}

impl VariableInfo {
    const NULL_ASSIGNMENT_LEVEL: usize = 0usize;
}
