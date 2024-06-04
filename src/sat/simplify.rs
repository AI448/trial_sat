use crate::finite_collections::{Array, FiniteMap};

use super::clause_theory::ClauseTheory;
use super::types::{Literal, Reason, VariableSize};
use super::variables::{VariableState, Variables};

#[derive(Default)]
pub struct Simplify {
    literal_buffer: Array<VariableSize, Literal>,
    decision_level_to_min_assignment_level: FiniteMap<VariableSize, VariableSize>,
    variable_index_to_redundancy: FiniteMap<VariableSize, bool>,
    literal_stack: Vec<Literal>,
}

impl Simplify {
    #[inline(never)]
    pub fn simplify(
        &mut self,
        clause: &mut Array<VariableSize, Literal>,
        variables: &Variables,
        theory: &mut ClauseTheory,
    ) {
        if clause.len() <= 2 {
            return;
        }
        // 割当レベルの昇順にソート
        clause.sort_by_cached_key(|l| match variables.get(l.index) {
            VariableState::Assigned { assignment_level, .. } => *assignment_level,
            VariableState::Unassigned { .. } => VariableSize::MAX,
            VariableState::TentativelyAssigned { .. } => {
                unreachable!()
            }
            VariableState::Conflicting { .. } => {
                unreachable!()
            }
        });
        //
        self.decision_level_to_min_assignment_level.clear();
        if variables.dimension() > self.decision_level_to_min_assignment_level.capacity() {
            self.decision_level_to_min_assignment_level
                .reserve(variables.dimension() - self.decision_level_to_min_assignment_level.capacity());
        }
        self.variable_index_to_redundancy.clear();
        if variables.dimension() > self.variable_index_to_redundancy.capacity() {
            self.variable_index_to_redundancy
                .reserve(variables.dimension() - self.variable_index_to_redundancy.capacity());
        }
        for literal in clause.iter() {
            if let VariableState::Assigned { assigned_value, decision_level, assignment_level, .. } =
                variables.get(literal.index)
            {
                debug_assert!(*assigned_value != literal.sign);
                if *assignment_level
                    < *self.decision_level_to_min_assignment_level.get(*decision_level).unwrap_or(&VariableSize::MAX)
                {
                    self.decision_level_to_min_assignment_level.insert(*decision_level, *assignment_level);
                }
                self.variable_index_to_redundancy.insert(literal.index, true);
            }
        }
        //
        let mut k = clause.len();
        while k != 0 {
            k -= 1;
            let literal = clause[k];
            if let VariableState::Assigned { assigned_value, .. } = variables.get(literal.index) {
                debug_assert!(*assigned_value != literal.sign);
                self.variable_index_to_redundancy.remove(literal.index);
                self.literal_stack.clear();
                if self.is_redundant(literal.index, variables, theory) {
                    clause.swap_remove(k);
                } else {
                    debug_assert!(self.variable_index_to_redundancy.contains_key(literal.index));
                }
                debug_assert!(self.literal_stack.is_empty());
            }
        }
    }

    fn is_redundant(&mut self, variable_index: VariableSize, variables: &Variables, theory: &mut ClauseTheory) -> bool {
        if let Some(is_redundant) = self.variable_index_to_redundancy.get(variable_index) {
            // 当該変数がキャッシュに含まれていればキャッシュの内容を返却
            return *is_redundant;
        }
        let mut is_redundant = true;
        if let VariableState::Assigned { assigned_value, decision_level, assignment_level, reason } =
            variables.get(variable_index)
        {
            if *decision_level == 0 {
                // 決定レベルが 0 ならば true
                is_redundant = true;
            } else if *assignment_level
                <= *self.decision_level_to_min_assignment_level.get(*decision_level).unwrap_or(&VariableSize::MAX)
            {
                // 当該変数の割当レベルが decision_level ごとの最小割当レベル以下ならば false
                is_redundant = false;
            } else if let Reason::Decision { .. } = reason {
                // 当該変数が決定変数ならば false
                is_redundant = false;
            } else if let Reason::Propagation { .. } = reason {
                // 当該変数の割当を説明する節を取得
                self.literal_buffer.clear();
                theory.explain(variable_index, *assigned_value, *reason, &mut self.literal_buffer);
                // 現在のスタックサイズを取得
                let n = self.literal_stack.len();
                // 当該変数以外の変数(当該変数への割当の原因になっている変数)をスタックに積む
                self.literal_stack.extend(self.literal_buffer.iter().filter(|l| l.index != variable_index));
                // 割当の原因になっている全変数について再帰して判定
                for k in n..self.literal_stack.len() {
                    is_redundant &= self.is_redundant(self.literal_stack[k].index, variables, theory);
                    if !is_redundant {
                        break;
                    }
                }
                // スタックをもとに戻す
                self.literal_stack.truncate(n);
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        }
        // 判定結果をキャッシュ(次回の探索時の枝狩りのため)
        self.variable_index_to_redundancy.insert(variable_index, is_redundant);
        // 判定結果を返却
        is_redundant
    }
}
