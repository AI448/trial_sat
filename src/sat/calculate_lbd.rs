use crate::finite_collections::{Array, Set};

use super::types::{Literal, VariableSize};
use super::variables::{Variable, Variables};

#[derive(Default)]
pub struct CalculateLBD {
    decision_level_set: Set<VariableSize>,
}

impl CalculateLBD {
    #[inline(never)]
    pub fn calculate(&mut self, literals: &Array<VariableSize, Literal>, variables: &Variables) -> VariableSize {
        self.decision_level_set.clear();
        if self.decision_level_set.capacity() < variables.dimension() {
            self.decision_level_set.reserve(variables.dimension() - self.decision_level_set.capacity());
        }
        let mut lbd: VariableSize = 0;
        for literal in literals.iter() {
            if let Variable::Assigned(assigned_variable) = variables.get(literal.index) {
                if *assigned_variable.decision_level() != 0
                    && !self.decision_level_set.contains_key(*assigned_variable.decision_level())
                {
                    self.decision_level_set.insert(*assigned_variable.decision_level());
                    lbd += 1;
                }
            }
        }
        self.decision_level_set.clear();
        lbd
    }
}
