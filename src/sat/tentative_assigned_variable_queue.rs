

use std::cmp::Ordering;
use super::variable_manager::Reason;
use super::super::finite_collections::FiniteHeapedMap;
use super::super::finite_collections::finite_heaped_map::Comparator;


/// 仮割当変数の優先度付きキュー
/// 矛盾の検知もこの構造体で行う
#[derive(Default)]
pub struct TentativeAssignedVariableQueue {
    // TODO: conflicting_variable_queue の要素は順序付けできないのでただの map でいい
    conflicting_variables: FiniteHeapedMap<[Reason; 2], ConflictingVariableComparator>,    
    consistent_variables: FiniteHeapedMap<(bool, Reason), ConsistentVariableComparator>,
}

impl TentativeAssignedVariableQueue {

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.conflicting_variables.capacity()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.conflicting_variables.is_empty() && self.consistent_variables.is_empty()
    }

    #[inline(never)]
    pub fn reserve(&mut self, additional: usize) {
        self.conflicting_variables.reserve(additional);
        self.consistent_variables.reserve(additional);
        assert!(self.conflicting_variables.capacity() == self.consistent_variables.capacity());
    }

    #[inline(always)]
    pub fn has_conflict(&self) -> bool {
        !self.conflicting_variables.is_empty()
    }

    #[inline(always)]
    pub fn iter_conflicting_variables(&self) -> impl Iterator<Item = &(usize, [Reason; 2])> {
        self.conflicting_variables.iter()
    }

    #[inline(never)]
    pub fn pop_first_consistent_variable(&mut self) -> Option<(usize, bool, Reason)> {
        match self.consistent_variables.pop_first() {
            Some((variable_index, (value, reason))) => Some((variable_index, value, reason)),
            None => None
        }
    }

    #[inline(never)]
    pub fn insert(&mut self, variable_index: usize, value: bool, reason: Reason) {
        if self.conflicting_variables.contains_key(variable_index) {
            // variable_index が既にに矛盾している場合
            let original_reasons = self.conflicting_variables.get(variable_index).unwrap();
            if TentativeAssignedVariableComparator::compare(&reason, &original_reasons[value as usize]) == Ordering::Less {
                // reason の順序が original よりも前である場合にはキューの内容を更新                
                let new_reasons;
                if value == false {
                    new_reasons = [reason, original_reasons[1].clone()];
                } else {
                    new_reasons = [original_reasons[0].clone(),  reason];
                }
                self.conflicting_variables.insert(variable_index, new_reasons);
            }
        } else {
            // 矛盾が発生していない場合
            if self.consistent_variables.contains_key(variable_index) {
                // variable_index が既に consisitent_variables に含まれている場合
                let (original_value, original_reason) = self.consistent_variables.get(variable_index).unwrap();
                if *original_value != value {
                    // 矛盾が発生する場合には conflicting_variables に変数を追加して sonsistent_variables からは削除
                    if value == false {
                        self.conflicting_variables.insert(variable_index, [reason, original_reason.clone()]);
                    } else{
                        self.conflicting_variables.insert(variable_index, [original_reason.clone(), reason]);
                    }
                    self.consistent_variables.remove(variable_index);
                } else if TentativeAssignedVariableComparator::compare(&reason, &original_reason) == Ordering::Less {
                    // 矛盾が発生せず reason の順序が original よりも前である場合にはキューの内容を更新
                    self.consistent_variables.insert(variable_index, (value, reason));
                }
            } else {
                // variable_index が consistent_variables に含まれていない場合には単に追加
                self.consistent_variables.insert(variable_index, (value, reason));
            }
        }
    }

    #[inline(never)]
    pub fn clear(&mut self) {
        self.conflicting_variables.clear();
        self.consistent_variables.clear();
    }

}


pub struct TentativeAssignedVariableComparator {
    // TODO: cmp は面倒なので Reason -> tuple の変換型をカスタマイズできる設計のほうが良さそう
}

impl TentativeAssignedVariableComparator {

    #[inline(always)]
    fn reason_to_tuple(reason: &Reason) -> (usize, usize) {
        match reason {
            Reason::Decision => (0, 0),
            Reason::Propagation {assignment_level_at_propagated , ..} => (1, *assignment_level_at_propagated)
        }
    }

    #[inline(always)]
    fn compare(lhs: &Reason, rhs: &Reason) -> Ordering {
        // TODO: ここのパフォーマンスは要確認．重たければタプルを事前に計算してヒープに持たせておく
        let l = Self::reason_to_tuple(lhs);
        let r = Self::reason_to_tuple(rhs);
        l.cmp(&r)
    }

}

struct ConsistentVariableComparator {}

impl Comparator<(bool, Reason)> for ConsistentVariableComparator {

    #[inline(always)]
    fn compare(lhs: &(usize, (bool, Reason)), rhs: &(usize, (bool, Reason))) -> Ordering {
        TentativeAssignedVariableComparator::compare(&lhs.1.1, &rhs.1.1)
    }
}

struct ConflictingVariableComparator {}

impl Comparator<[Reason; 2]> for ConflictingVariableComparator {

    #[inline(always)]
    fn compare(_: &(usize, [Reason; 2]), _: &(usize, [Reason; 2])) -> Ordering {
        Ordering::Equal
    }
}

