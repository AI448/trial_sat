use crate::finite_collections::{Array, Comparator, FiniteHeapedMap};

use super::clause_theory::ClauseTheory;
use super::simplify::Simplify;
use super::types::{Literal, Reason, VariableSize};
use super::variables::{Variable, Variables};

struct AnalyzerBufferValue {
    // TODO 検討 豪華すぎるので削減してもいいかも
    sign: bool,
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
    simplify: Simplify,
    literals: Array<VariableSize, Literal>,
    analyzer_buffer: FiniteHeapedMap<VariableSize, AnalyzerBufferValue, AnalyzerBufferComparator>,
}

impl Analyze {
    #[inline(never)]
    pub fn analyze(
        &mut self,
        conflicting_variable_index: VariableSize,
        reasons: [Reason; 2],
        variables: &mut Variables,
        theory: &mut ClauseTheory,
    ) -> (VariableSize, Array<VariableSize, Literal>) {
        self.analyzer_buffer.clear();
        if self.analyzer_buffer.capacity() < variables.dimension() {
            self.analyzer_buffer.reserve(variables.dimension() - self.analyzer_buffer.capacity());
        }
        // 矛盾している 2 つの節を融合
        for (reason, value) in reasons.iter().zip([false, true]) {
            // 矛盾が生じている変数のアクティビティを増大
            variables.increase_activity(conflicting_variable_index);
            self.resolve(conflicting_variable_index, value, *reason, variables, theory);
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
                    self.simplify.simplify(&mut learnt_clause, variables, theory);
                    // 学習節に含まれる変数のアクティビティを増大
                    for literal in learnt_clause.iter() {
                        variables.increase_activity(literal.index);
                    }
                    return (*second_largest_decision_level, learnt_clause);
                }
            }
            // 最大割り当てレベル(節融合による消去対象)の変数を選択
            let variable_index = self.analyzer_buffer.first_key_value().unwrap().0;
            let value = !self.analyzer_buffer.first_key_value().unwrap().1.sign;
            let reason = self.analyzer_buffer.first_key_value().unwrap().1.reason;
            // 消去対象の変数のアクティビティを増大
            variables.increase_activity(variable_index);
            // 節融合
            self.resolve(variable_index, value, reason, variables, theory);
        }
    }

    #[inline(never)]
    fn resolve(
        &mut self,
        variable_index: VariableSize,
        value: bool,
        reason: Reason,
        variables: &Variables,
        theory: &mut ClauseTheory,
    ) {
        // 割り当てを説明する節を取得
        self.literals.clear();
        theory.explain(variable_index, value, reason, &mut self.literals);
        // 節を analyzer_buffer に融合
        for literal in self.literals.iter() {
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
                if let Variable::Assigned(assigned_variable) = variables.get(literal.index) {
                    debug_assert!(*assigned_variable.value() == !literal.sign); // 偽が割り当てられているはず
                    if *assigned_variable.decision_level() != 0 {
                        self.analyzer_buffer.insert(
                            literal.index,
                            AnalyzerBufferValue {
                                sign: literal.sign,
                                decision_level: *assigned_variable.decision_level(),
                                assignment_level: *assigned_variable.assignment_level(),
                                reason: *assigned_variable.reason(),
                            },
                        );
                    }
                }
            }
        }
    }
}
