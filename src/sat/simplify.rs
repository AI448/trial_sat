use crate::finite_collections::{Array, FiniteMap};

use super::clause_theory::ClauseTheory;
use super::types::{Literal, Reason, VariableSize};
use super::variables::{Variable, Variables};

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
            Variable::Assigned(assigned_variable) => *assigned_variable.assignment_level(),
            Variable::Notassigned(..) => VariableSize::MAX,
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
            if let Variable::Assigned(variable) = variables.get(literal.index) {
                debug_assert!(*variable.value() != literal.sign);
                if *variable.assignment_level()
                    < *self
                        .decision_level_to_min_assignment_level
                        .get(*variable.decision_level())
                        .unwrap_or(&VariableSize::MAX)
                {
                    self.decision_level_to_min_assignment_level
                        .insert(*variable.decision_level(), *variable.assignment_level());
                }
                self.variable_index_to_redundancy.insert(literal.index, true);
            }
        }
        //
        let mut k = clause.len();
        while k != 0 {
            k -= 1;
            let literal = clause[k];
            if let Variable::Assigned(variable) = variables.get(literal.index) {
                debug_assert!(*variable.value() != literal.sign);
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
        if let Variable::Assigned(variable) = variables.get(variable_index) {
            if *variable.decision_level() == 0 {
                // 決定レベルが 0 ならば true
                is_redundant = true;
            } else if *variable.assignment_level()
                <= *self
                    .decision_level_to_min_assignment_level
                    .get(*variable.decision_level())
                    .unwrap_or(&VariableSize::MAX)
            {
                // 当該変数の割当レベルが decision_level ごとの最小割当レベル以下ならば false
                is_redundant = false;
            } else if let Reason::Decision { .. } = variable.reason() {
                // 当該変数が決定変数ならば false
                is_redundant = false;
            } else if let Reason::Propagation { .. } = variable.reason() {
                // 当該変数の割当を説明する節を取得
                self.literal_buffer.clear();
                theory.explain(variable_index, *variable.value(), *variable.reason(), &mut self.literal_buffer);
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
