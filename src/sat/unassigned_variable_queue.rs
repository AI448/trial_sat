
use crate::finite_collections::{Array, FiniteHeapedMap, Comparator};
use super::types::VariableSize;

/// 未割り当て変数の優先度付きキュー
pub struct UnassignedVariableQueue {
    time_constant: f64,
    increase_value: f64,
    activities: Array<VariableSize, f64>,
    queue: FiniteHeapedMap<VariableSize, f64, UnassignedVariableComparator>,
}

impl UnassignedVariableQueue {
    pub fn new(time_constant: f64) -> Self {
        UnassignedVariableQueue {
            time_constant: time_constant,
            increase_value: 1.0,
            activities: Array::default(),
            queue: FiniteHeapedMap::default(),
        }
    }

    pub fn capacity(&self) -> VariableSize {
        assert!(self.activities.len() == self.queue.capacity());
        self.activities.len()
    }

    pub fn is_empty(&self) -> bool {
        assert!(self.activities.is_empty() == self.queue.is_empty());
        self.activities.is_empty()
    }

    pub fn reserve(&mut self, additional: VariableSize) {
        self.activities.resize(self.activities.len() + additional, 0.0);
        self.queue.reserve(additional);
    }

    pub fn insert(&mut self, variable_index: VariableSize) {
        self.queue.insert(variable_index, self.activities[variable_index]);
    }

    pub fn pop_first(&mut self) -> Option<VariableSize> {
        match self.queue.pop_first() {
            Some((variable_index, ..)) => Some(variable_index),
            None => None,
        }
    }

    pub fn increase_activity(&mut self, variable_index: VariableSize) {
        self.activities[variable_index] += self.increase_value;
        if self.queue.contains_key(variable_index) {
            self.queue.insert(variable_index, self.activities[variable_index]);
        }
    }

    pub fn advance_time(&mut self) {
        self.increase_value /= 1.0 - 1.0 / self.time_constant;
        if self.increase_value > 1e4 {
            for activity in self.activities.iter_mut() {
                *activity /= self.increase_value;
            }
            self.increase_value = 1.0;
            // MEMO: queue に entry を作れば効率的だが，全体的なパフォーマンスには大して影響しないので気が向いたときに対応
            let variable_indices = Vec::from_iter(self.queue.iter().map(|x| x.0));
            self.queue.clear();
            for variable_index in variable_indices {
                self.queue.insert(variable_index, self.activities[variable_index]);
            }
        }
    }
}

pub struct UnassignedVariableComparator {}

impl Comparator<VariableSize, f64> for UnassignedVariableComparator {
    #[inline(always)]
    fn compare(lhs: &(VariableSize, f64), rhs: &(VariableSize, f64)) -> std::cmp::Ordering {
        rhs.1.partial_cmp(&lhs.1).unwrap()
    }
}
