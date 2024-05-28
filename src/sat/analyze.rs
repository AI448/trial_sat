use crate::finite_collections::{Array, Comparator, FiniteHeapedMap};

use super::clause_theory::ClauseTheory;
use super::types::{ConstraintSize, Literal, VariableSize};
use super::unassigned_variable_queue::UnassignedVariableQueue;
use super::variable_manager::{Reason, VariableManager, VariableState};

struct AnalyzerBufferValue {
    sign: bool,
    value: bool,
    decision_level: VariableSize,
    assignment_level: VariableSize,
    reason: Reason,
}

struct AnalyzerBufferComparator {}

impl Comparator<VariableSize, AnalyzerBufferValue> for AnalyzerBufferComparator {
    #[inline(always)]
    fn compare(lhs: &(u32, AnalyzerBufferValue), rhs: &(u32, AnalyzerBufferValue)) -> std::cmp::Ordering {
        rhs.1.assignment_level.cmp(&lhs.1.assignment_level)
    }
}

#[derive(Default)]
pub struct Analyze {
    literal_buffer: Array<VariableSize, Literal>,
    analyzer_buffer: FiniteHeapedMap<VariableSize, AnalyzerBufferValue, AnalyzerBufferComparator>,
}

impl Analyze {
    #[inline(never)]
    pub fn analyze(
        &mut self,
        variables: &VariableManager,
        theory: &mut ClauseTheory, // TODO mut で渡すのかっこ悪い
        conflicting_variable_index: ConstraintSize,
        reasons: [Reason; 2],
        unassigned_variable_queue: &mut UnassignedVariableQueue, // TODO mut で渡すのかっこ悪い
    ) -> (VariableSize, Array<VariableSize, Literal>) {
        self.analyzer_buffer.clear();
        if self.analyzer_buffer.capacity() < variables.number_of_variables() {
            self.analyzer_buffer.reserve(variables.number_of_variables() - self.analyzer_buffer.capacity());
        }
        // 矛盾が生じている変数のアクティビティを増大
        unassigned_variable_queue.increase_activity(conflicting_variable_index);
        // 矛盾している 2 つの節を融合
        for (reason, value) in reasons.iter().zip([false, true]) {
            self.resolve(variables, theory, conflicting_variable_index, value, *reason);
        }
        // バックジャンプ可能な節が獲られるまで融合を繰り返す
        loop {
            if self.analyzer_buffer.len() == 0 {
                // 節融合の結果が空になった場合には空の学習節を返す(Unsatisifiable)
                return (0, Array::default());
            }
            {
                // バックジャンプ可能かを判定
                // 二分ヒープの先頭 3 つの decision_level を取得
                // NOTE: 二分ヒープなので，最大要素と 2 番目に大きい要素を使用するなら，最初の 3 要素だけを見れば十分．
                // TODO: 毎回メモリアロケーションが発生しているのが気になる
                let first_three_decision_levels =
                    Vec::from_iter(self.analyzer_buffer.iter().take(3).map(|item| item.1.decision_level));
                assert!(first_three_decision_levels.len() >= 1);
                // 最も大きい決定レベルは現在の決定レベルと同じであるはず
                assert!(first_three_decision_levels[0] == variables.current_decision_level());
                // 2 番目に大きい決定レベルを取得
                let second_largest_decision_level = first_three_decision_levels[1..].iter().max().unwrap_or(&0);
                if *second_largest_decision_level < variables.current_decision_level() {
                    // 2 番目に大きい決定レベルが現在の決定レベル未満であればバックジャンプ可能なので，現在の節を学習節として返す
                    let mut learnt_clause = Array::default();
                    learnt_clause.reserve(self.analyzer_buffer.len());
                    for (variable_index, buffer_value) in self.analyzer_buffer.iter() {
                        learnt_clause.push(Literal { index: *variable_index, sign: buffer_value.sign });
                    }
                    // simplify
                    self.simplify(variables, theory, &mut learnt_clause);
                    // 学習節に含まれる変数のアクティビティを増大
                    for literal in learnt_clause.iter() {
                        unassigned_variable_queue.increase_activity(literal.index);
                    }
                    return (*second_largest_decision_level, learnt_clause);
                }
            }
            // 最大割り当てレベル(節融合による消去対象)の変数を選択
            let variable_index = self.analyzer_buffer.first_key_value().unwrap().0;
            let value = !self.analyzer_buffer.first_key_value().unwrap().1.sign;
            let reason = self.analyzer_buffer.first_key_value().unwrap().1.reason;
            // 消去対象の変数のアクティビティを増大
            unassigned_variable_queue.increase_activity(variable_index);
            // 節融合
            self.resolve(variables, theory, variable_index, value, reason);
        }
    }

    #[inline(never)]
    fn resolve(
        &mut self,
        variable_manager: &VariableManager,
        theory: &mut ClauseTheory,
        variable_index: VariableSize,
        value: bool,
        reason: Reason,
    ) {
        // 割り当てを説明する節を取得
        self.literal_buffer.clear();
        theory.explain(variable_index, value, reason, &mut self.literal_buffer, true);
        // 節を analyzer_buffer に融合
        for literal in self.literal_buffer.iter() {
            if self.analyzer_buffer.contains_key(literal.index) {
                if self.analyzer_buffer.get(literal.index).unwrap().sign == !literal.sign {
                    // 逆符号のリテラルが含まれていればそれを削除
                    self.analyzer_buffer.remove(literal.index);
                } else {
                    // 同符号のリテラルが含まれていれば何もしない
                }
            } else {
                // リテラルが含まれていない場合
                // リテラルが割り当て済みかつ割り当てレベルが非零ならそのリテラルを追加
                if let VariableState::Assigned { value, decision_level, assignment_level, reason } =
                    variable_manager.get_state(literal.index)
                {
                    debug_assert!(value == !literal.sign); // 偽が割り当てられているはず
                    if decision_level != 0 {
                        self.analyzer_buffer.insert(
                            literal.index,
                            AnalyzerBufferValue {
                                sign: literal.sign,
                                value: value,
                                decision_level: decision_level,
                                assignment_level: assignment_level,
                                reason: reason,
                            },
                        );
                    }
                }
            }
        }
    }

    fn simplify(
        &mut self,
        variables: &VariableManager,
        theory: &mut ClauseTheory,
        literals: &mut Array<VariableSize, Literal>,
    ) {
        let n = literals.len();
        // 割当済みの変数を analyzer_buffer に移動(決定レベル 0 で割当済みであれば単に削除)
        self.analyzer_buffer.clear();
        let mut k: VariableSize = 1;
        while k < literals.len() {
            let literal = literals[k];
            if let VariableState::Assigned { value, decision_level, assignment_level, reason, .. } =
                variables.get_state(literal.index)
            {
                if decision_level != 0 {
                    self.analyzer_buffer.insert(
                        literal.index,
                        AnalyzerBufferValue { sign: literal.sign, value, decision_level, assignment_level, reason },
                    );
                }
                literals.swap_remove(k);
            } else {
                k += 1;
            }
        }
        //
        while !self.analyzer_buffer.is_empty() {
            let (variable_index, heap_item) = self.analyzer_buffer.pop_first().unwrap();
            if let Reason::Propagation { .. } = heap_item.reason {
                self.literal_buffer.clear();
                // TODO: 確認 制約のアクティビティが加算されてしまうが大丈夫か
                theory.explain(variable_index, heap_item.value, heap_item.reason, &mut self.literal_buffer, false);
                //
                let is_erasable = self.literal_buffer.iter().all(|l| {
                    if l.index == variable_index {
                        debug_assert!(l.sign == heap_item.value);
                        true
                    } else {
                        self.analyzer_buffer.get(l.index).is_some_and(|v| v.sign == l.sign)
                    }
                });
                if !is_erasable {
                    literals.push(Literal { index: variable_index, sign: heap_item.sign });
                }
            } else {
                literals.push(Literal { index: variable_index, sign: heap_item.sign });
            }
        }
        if literals.len() < n {
            // eprintln!("simpify {} -> {}", n, literals.len());
        }
    }
}
