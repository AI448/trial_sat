use crate::finite_collections::Array;
use average::{AverageTrait, ExponentialMovingAverage, MovingAverage};

use super::calculate_lbd::CalculateLBD;
use super::types::{ConstraintSize, Literal, Reason, VariableSize};
use super::variables::{VariableState, Variables};

#[derive(Clone, Copy)]
struct WatchedBy {
    clause_index: ConstraintSize,
    watching_position: VariableSize,
    cached_another_literal: Option<Literal>,
}

struct Clause {
    literals: Array<VariableSize, Literal>,
    // TODO 節の状態は「非学習節・学習節・削除された学習節」のいずれかなので，以下は enum にした方がきれい
    is_learnt: bool,
    lbd: VariableSize,
    is_deleted: bool,
    generated_time: usize,
    last_used_time: usize,
    activity: f64,
}

pub struct ClauseTheory {
    activity_time_constant: f64,
    activity_increase_value: f64,
    watched_infos: Array<VariableSize, [Array<ConstraintSize, WatchedBy>; 2]>, // NOTE: Literal を添え字にしてアクセスできる配列を使いたい
    clause_infos: Array<ConstraintSize, Clause>,
    calculate_lbd: CalculateLBD,
    time: usize,
    lbd_average: ExponentialMovingAverage<f64>,
    current_lbd_average: MovingAverage<f64>,
    last_reduction_time: usize,
    check_count: usize,
    skip_by_cached_count: usize,
    skip_by_another_count: usize,
    propagation_count: usize,
    clause_reduction_count: usize,
}

impl ClauseTheory {
    pub fn new(
        lbd_averaging_time_constant: usize,
        current_lbd_averaging_time_constant: usize,
        clause_activity_time_constant: usize,
    ) -> Self {
        ClauseTheory {
            activity_time_constant: clause_activity_time_constant as f64,
            activity_increase_value: 1.0,
            watched_infos: Array::default(),
            clause_infos: Array::default(),
            calculate_lbd: CalculateLBD::default(),
            time: 0,
            lbd_average: ExponentialMovingAverage::new(lbd_averaging_time_constant),
            current_lbd_average: MovingAverage::new(current_lbd_averaging_time_constant),
            last_reduction_time: 0,
            check_count: 0,
            skip_by_cached_count: 0,
            skip_by_another_count: 0,
            propagation_count: 0,
            clause_reduction_count: 0,
        }
    }

    pub fn expand(&mut self, additional: VariableSize) {
        self.watched_infos.resize_with(self.watched_infos.len() + additional, || [Array::default(), Array::default()]);
    }

    pub fn add_clause(
        &mut self,
        mut literals: Array<VariableSize, Literal>,
        is_learnt: bool,
        variables: &mut Variables,
    ) {
        // TODO: あとで対応(すべてのリテラルに偽が割り当てられているケースはひとまず考えない)
        debug_assert!(!literals.iter().all(|literal| variables.get(literal.index).is_value_assigned(!literal.sign)));
        let clause_index = self.clause_infos.len();

        let lbd;
        if literals.len() == 0 {
            debug_assert!(false); // TODO: あとで対応(上の all での判定で除かれるはず)
            lbd = 0;
        } else if literals.len() == 1 {
            lbd = 0;
            if !variables.get(literals[0].index).is_assigned() {
                variables.tentatively_assign(
                    literals[0].index,
                    literals[0].sign,
                    Reason::Propagation {
                        clause_index: clause_index,
                        lbd: lbd,
                        clause_length: 1,
                        assignment_level_at_propagated: variables.current_assignment_level(),
                    },
                );
            }
        } else {
            lbd = if is_learnt { self.calculate_lbd.calculate(&literals, variables) } else { literals.len() };

            /* 割当の状態に応じてリテラルをソート
             * 1. 真が割り当てられている -> 未割り当て -> 偽が割り当てられているの順
             * 2. 真が割り当てられているリテラル同士では割当レベルの昇順
             *    偽が割り当てられているリテラル同士では割当レベルの降順
             */
            literals.sort_by_cached_key(|l| match variables.get(l.index) {
                VariableState::Assigned { assigned_value, assignment_level, .. } => {
                    if *assigned_value == l.sign {
                        (0, *assignment_level)
                    } else {
                        (2, VariableSize::MAX - *assignment_level)
                    }
                }
                VariableState::Unassigned { .. } => (1, 0),
                VariableState::TentativelyAssigned { .. } => (1, 0),
                VariableState::Conflicting { .. } => (1, 0),
            });
            // 少なくとも先頭要素に偽が割り当てられていることはないはず
            debug_assert!(!variables.get(literals[0].index).is_value_assigned(!literals[0].sign));
            // 先頭の 2 つを監視リテラルに
            for (k, literal) in literals.iter().enumerate().take(2) {
                self.watched_infos[literal.index][1 - (literal.sign as usize)].push(WatchedBy {
                    clause_index: clause_index,
                    watching_position: k as VariableSize,
                    cached_another_literal: Some(literals[1 - k as VariableSize]),
                });
            }

            if variables.get(literals[1].index).is_value_assigned(!literals[1].sign) {
                // 末尾の監視リテラルに偽が割り当てられている場合には未割り当ての監視リテラルに真を割り当て
                if !variables.get(literals[0].index).is_assigned() {
                    //
                    let mut lbd_upper = lbd;
                    if variables.current_decision_level() != 0 {
                        for literal in literals.iter() {
                            if let VariableState::Assigned { decision_level, reason, .. } = variables.get(literal.index)
                            {
                                if *decision_level == variables.current_decision_level() {
                                    if let Reason::Propagation { lbd: u, .. } = reason {
                                        debug_assert!(*u >= 1);
                                        debug_assert!(*u <= variables.current_decision_level());
                                        lbd_upper += u - 1;
                                    }
                                }
                            }
                        }
                        lbd_upper = lbd_upper.min(variables.current_decision_level());
                    }
                    variables.tentatively_assign(
                        literals[0].index,
                        literals[0].sign,
                        Reason::Propagation {
                            clause_index: clause_index,
                            lbd: lbd_upper,
                            clause_length: literals.len(),
                            assignment_level_at_propagated: variables.current_assignment_level(),
                        },
                    );
                }
            }
        }
        //
        debug_assert!(literals.len() <= 1 || lbd >= 1);
        if is_learnt {
            self.lbd_average.add(lbd as f64);
            self.current_lbd_average.add(lbd as f64);
        }
        // 節を追加
        self.clause_infos.push(Clause {
            literals: literals,
            is_learnt: is_learnt,
            lbd: lbd,
            is_deleted: false,
            generated_time: self.time,
            last_used_time: self.time,
            activity: self.activity_increase_value,
        });
    }

    pub fn propagate(&mut self, assigned_variable_index: VariableSize, variables: &mut Variables) {
        let VariableState::Assigned { assigned_value, .. } = *variables.get(assigned_variable_index) else {
            unreachable!();
        };
        let mut k: ConstraintSize = 0;
        // variable_index への value の割当を監視している節を走査
        'loop_watching_clause: while k < self.watched_infos[assigned_variable_index][assigned_value as usize].len() {
            self.check_count += 1;
            let WatchedBy { clause_index, watching_position, cached_another_literal } =
                self.watched_infos[assigned_variable_index][assigned_value as usize][k];
            // println!("c{}", clause_index);
            debug_assert!(watching_position < 2);
            if cached_another_literal.is_some_and(|l| variables.get(l.index).is_value_assigned(l.sign)) {
                // cached_another_literal に真が割り当てられており既に充足されているのでなにもしない
                self.skip_by_cached_count += 1;
            } else {
                let clause = &mut self.clause_infos[clause_index];
                let watched_literal = clause.literals[watching_position];
                debug_assert!(watched_literal.index == assigned_variable_index);
                debug_assert!(watched_literal.sign == !assigned_value);
                let another_watched_literal = clause.literals[1 - watching_position];
                if variables.get(another_watched_literal.index).is_value_assigned(another_watched_literal.sign) {
                    // もう一方の監視リテラルに真が割り当てられており既に充足されている場合
                    self.skip_by_another_count += 1;
                    // another_watched_literal をキャッシュしておく
                    self.watched_infos[assigned_variable_index][assigned_value as usize][k].cached_another_literal =
                        Some(another_watched_literal);
                } else {
                    // 監視対象ではないリテラルを走査
                    for (l, literal) in clause.literals.iter().enumerate().skip(2) {
                        if !variables.get(literal.index).is_value_assigned(!literal.sign) {
                            // 真が割り当てられているまたは未割り当てのリテラルを発見した場合
                            // 元の監視リテラルの監視を解除
                            self.watched_infos[watched_literal.index][!watched_literal.sign as usize].swap_remove(k);
                            // 発見したリテラルを監視
                            self.watched_infos[literal.index][!literal.sign as usize].push(WatchedBy {
                                clause_index: clause_index,
                                watching_position,
                                cached_another_literal: None,
                            });
                            // 発見したリテラルを監視位置に移動
                            clause.literals.swap(watching_position, l as VariableSize);
                            // 次の節へ
                            continue 'loop_watching_clause;
                        }
                    }
                    // 真が割り当てられているまたは未割り当てのリテラルが見つからなかった場合
                    debug_assert!(!variables
                        .get(another_watched_literal.index)
                        .is_value_assigned(!another_watched_literal.sign)); // もう一方の監視リテラルに false が割り当てられていることはないはず

                    self.propagation_count += 1;
                    // plbd を計算
                    let lbd = self.calculate_lbd.calculate(&clause.literals, variables);
                    if clause.is_learnt && lbd < clause.lbd {
                        clause.lbd = lbd;
                    }
                    //
                    let mut lbd_upper = lbd;
                    if variables.current_decision_level() != 0 {
                        for literal in clause.literals.iter() {
                            if let VariableState::Assigned { decision_level, reason, .. } = variables.get(literal.index)
                            {
                                if *decision_level == variables.current_decision_level() {
                                    if let Reason::Propagation { lbd: u, .. } = reason {
                                        debug_assert!(*u >= 1);
                                        debug_assert!(*u <= variables.current_decision_level());
                                        lbd_upper += u - 1;
                                    }
                                }
                            }
                        }
                        lbd_upper = lbd_upper.min(variables.current_decision_level());
                    }
                    // もう一方の監視リテラルに真を割り当て
                    variables.tentatively_assign(
                        another_watched_literal.index,
                        another_watched_literal.sign,
                        Reason::Propagation {
                            clause_index: clause_index,
                            lbd: lbd_upper,
                            clause_length: clause.literals.len(),
                            assignment_level_at_propagated: variables.current_assignment_level(),
                        },
                    );
                }
            }
            k += 1;
        }
    }

    pub fn explain(
        &self,
        variable_index: VariableSize,
        value: bool,
        reason: Reason,
        clause: &mut Array<VariableSize, Literal>,
    ) {
        assert!(matches!(reason, Reason::Propagation { .. }));
        let Reason::Propagation { clause_index, .. } = reason else {
            unreachable!();
        };
        assert!(self.clause_infos[clause_index].literals.iter().any(|l| l.index == variable_index && l.sign == value));
        clause.clone_from(&self.clause_infos[clause_index].literals);
    }

    pub fn increase_activity(&mut self, clause_index: ConstraintSize) {
        self.clause_infos[clause_index].activity += self.activity_increase_value;
        self.clause_infos[clause_index].last_used_time = self.time;
    }

    pub fn advance_time(&mut self) {
        self.time += 1;
        self.activity_increase_value /= 1.0 - 1.0 / self.activity_time_constant;
        if self.activity_increase_value > 1e4 {
            for clause in self.clause_infos.iter_mut() {
                if clause.is_learnt && !clause.is_deleted {
                    clause.activity /= self.activity_increase_value;
                }
            }
            self.activity_increase_value = 1.0;
        }
    }

    pub fn is_request_restart(&self) -> bool {
        let lbd_is_too_large = self.current_lbd_average.value() * self.current_lbd_average.count() as f64
            / self.current_lbd_average.time_constant() as f64
            > self.lbd_average.value();
        let restart_interval_is_too_long =
            self.time > self.last_reduction_time + 2 * (5000 + 100 * self.clause_reduction_count);
        lbd_is_too_large || restart_interval_is_too_long
    }

    pub fn restart(&mut self, variables: &Variables) {
        assert!(variables.current_decision_level() == 0);
        eprintln!(
            "pldb_average={} current_pldb_average={}",
            self.lbd_average.value(),
            self.current_lbd_average.value()
        );
        self.current_lbd_average.clear();

        if self.clause_reduction_count == 0
            || self.time > self.last_reduction_time + 5000 + 100 * self.clause_reduction_count
        {
            // 決定レベル 0 で充足されている節を削除
            for clause in self.clause_infos.iter_mut().filter(|c| !c.is_deleted) {
                let satisfied = clause.literals.iter().any(|l| variables.get(l.index).is_value_assigned(l.sign));
                if satisfied {
                    clause.is_deleted = true;
                    clause.literals.clear();
                    clause.literals.shrink_to_fit();
                } else {
                    // fix されている変数を節から削除(2 つ目のリテラルまでは監視対象かもしれないのでひとまず触らない)
                    let mut k = 2;
                    while k < clause.literals.len() {
                        if variables.get(clause.literals[k].index).is_value_assigned(!clause.literals[k].sign) {
                            clause.literals.swap_remove(k);
                        } else {
                            k += 1;
                        }
                    }
                }
            }

            // 削除対象の候補を列挙
            let mut clause_priority_order = Vec::from_iter((0..self.clause_infos.len()).filter(|i| {
                self.clause_infos[*i].is_learnt
                    && !self.clause_infos[*i].is_deleted
                    && self.clause_infos[*i].lbd >= 3
                    && (self.clause_infos[*i].lbd >= 6
                        || self.clause_infos[*i].last_used_time < self.last_reduction_time)
                    && self.clause_infos[*i].generated_time < self.last_reduction_time
            }));
            // 削除の優先度の高い順にソート
            clause_priority_order.sort_unstable_by(|l, r| {
                let lhs = self.clause_infos[*l].activity;
                let rhs = self.clause_infos[*r].activity;
                lhs.partial_cmp(&rhs).unwrap()
            });
            // 1/2 削除
            for clause_index in clause_priority_order.iter().take(clause_priority_order.len() / 2) {
                let clause = &mut self.clause_infos[*clause_index];
                clause.is_deleted = true;
                clause.literals.clear();
                clause.literals.shrink_to_fit();
            }
            // 削除された節の監視を削除
            for variable_index in 0..self.watched_infos.len() {
                for value in [0, 1] {
                    let list = &mut self.watched_infos[variable_index][value];
                    let mut k: ConstraintSize = 0;
                    while k < list.len() {
                        if self.clause_infos[list[k].clause_index].is_deleted {
                            list.swap_remove(k);
                        } else {
                            k += 1;
                        }
                    }
                }
            }
            {
                // デバッグ出力
                let mut number_of_learnt_clauses = 0;
                let mut lbd_ammount = 0;
                for clause in self.clause_infos.iter() {
                    if clause.is_learnt && !clause.is_deleted {
                        number_of_learnt_clauses += 1;
                        lbd_ammount += clause.lbd;
                    }
                }
                eprintln!(
                    "reduce learnt clauses {} pldb_average={}",
                    number_of_learnt_clauses,
                    lbd_ammount as f64 / number_of_learnt_clauses as f64
                );
            }
            self.clause_reduction_count += 1;
            self.last_reduction_time = self.time;
        }
    }

    pub fn summary(&self) -> (usize, usize, usize, usize) {
        (self.check_count, self.skip_by_cached_count, self.skip_by_another_count, self.propagation_count)
    }
}
