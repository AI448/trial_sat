use super::super::finite_collections;
use super::types::Literal;
use super::variable_manager::{VariableManager, VariableState};


#[derive(Default)]
pub struct CalculatePseudoLBD {
    decision_level_set: finite_collections::Set,
}

impl CalculatePseudoLBD {

    pub fn calculate(&mut self, variable_manager: &VariableManager, literals: &Vec<Literal>) -> u64 {
        self.decision_level_set.clear();
        if self.decision_level_set.capacity() < variable_manager.number_of_variables() {
            self.decision_level_set.reserve(variable_manager.number_of_variables() - self.decision_level_set.capacity());
        }
        let mut pseudo_lbd = 0u64;
        for literal in literals.iter() {
            if let VariableState::Assigned {decision_level, ..} = variable_manager.get_state(literal.index) {
                if !self.decision_level_set.contains_key(decision_level) {
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