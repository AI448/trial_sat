use crate::finite_collections::{Array, Set};

use super::types::{VariableSize, Literal};
use super::variable_manager::{VariableManager, VariableState};

#[derive(Default)]
pub struct CalculatePseudoLBD {
    decision_level_set: Set<u32>,
}

impl CalculatePseudoLBD {
    #[inline(never)]
    pub fn calculate(&mut self, variable_manager: &VariableManager, literals: &Array<VariableSize, Literal>) -> VariableSize {
        self.decision_level_set.clear();
        if self.decision_level_set.capacity() < variable_manager.number_of_variables() {
            self.decision_level_set
                .reserve(variable_manager.number_of_variables() - self.decision_level_set.capacity());
        }
        let mut pseudo_lbd = 0 as VariableSize;
        for literal in literals.iter() {
            if let VariableState::Assigned { decision_level, .. } = variable_manager.get_state(literal.index) {
                if decision_level != 0 && !self.decision_level_set.contains_key(decision_level) {
                    self.decision_level_set.insert(decision_level);
                    pseudo_lbd += 1;
                }
            } else {
                pseudo_lbd += 1;
            }
        }
        self.decision_level_set.clear();
        pseudo_lbd
    }
}
