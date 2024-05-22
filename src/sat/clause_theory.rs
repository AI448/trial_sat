use std::collections::VecDeque;

use super::calculate_pseudo_lbd::CalculatePseudoLBD;
use super::tentative_assigned_variable_queue::TentativeAssignedVariableQueue;
use super::types::Literal;
use super::variable_manager::Reason;
use super::variable_manager::VariableManager;
use super::variable_manager::VariableState;

#[derive(Clone, Copy)]
struct WatchedBy {
    clause_index: usize,
    watching_position: usize,
}

struct Clause {
    literals: Vec<Literal>,
    // TODO 節の状態は「非学習節・学習節・削除された学習節」のいずれかなので，以下は enum にした方がきれい
    is_learnt: bool,
    plbd: u64,
    is_deleted: bool,
    activity: f64,
}

pub struct ClauseTheory {
    activity_time_constant: f64,
    activity_increase_value: f64,
    watched_infos: Vec<[Vec<WatchedBy>; 2]>, // NOTE: Literal を添え字にしてアクセスできる配列を使いたい
    clause_infos: Vec<Clause>,
    calculate_plbd: CalculatePseudoLBD,
    number_of_learnt_clauses: usize,
    plbd_ammount: u64,
    current_plbd_ammount: u64,
    current_plbd_deque: VecDeque<u64>,
    check_count: usize,
    skip_count: usize,
    propagation_count: usize,
    clause_reduction_count: usize,
}

impl ClauseTheory {
    pub fn new(activity_time_constant: f64) -> Self {
        ClauseTheory {
            activity_time_constant: activity_time_constant,
            activity_increase_value: 1.0,
            watched_infos: Vec::default(),
            clause_infos: Vec::default(),
            calculate_plbd: CalculatePseudoLBD::default(),
            number_of_learnt_clauses: 0,
            plbd_ammount: 0,
            current_plbd_ammount: 0,
            current_plbd_deque: VecDeque::default(),
            check_count: 0,
            skip_count: 0,
            propagation_count: 0,
            clause_reduction_count: 0,
        }
    }

    pub fn expand(&mut self, additional: usize) {
        self.watched_infos.resize_with(self.watched_infos.len() + additional, || [vec![], vec![]]);
    }

    pub fn add_clause(
        &mut self,
        variable_manager: &VariableManager,
        mut literals: Vec<Literal>,
        is_learnt: bool,
        tentative_assigned_variable_queue: &mut TentativeAssignedVariableQueue,
    ) {
        // TODO: あとで対応(すべてのリテラルに偽が割り当てられているケースはひとまず考えない)
        assert!(!literals.iter().all(|literal| variable_manager.is_false(*literal)));
        let clause_index = self.clause_infos.len();
        if literals.len() == 0 {
            assert!(false); // TODO: あとで対応(上の all での判定で除かれるはず)
        } else if literals.len() == 1 {
            if !variable_manager.is_assigned(literals[0]) {
                tentative_assigned_variable_queue.insert(
                    literals[0].index,
                    literals[0].sign,
                    Reason::Propagation {
                        clause_index: clause_index,
                        assignment_level_at_propagated: variable_manager.current_assignment_level(),
                    },
                );
            }
        } else {
            /* 割当の状態に応じてリテラルをソート
             * 1. 真が割り当てられている -> 未割り当て -> 偽が割り当てられているの順
             * 2. 真が割り当てられているリテラル同士では割当レベルの昇順
             *    偽が割り当てられているリテラル同士では割当レベルの降順
             */
            literals.sort_by_cached_key(|l| match variable_manager.get_state(l.index) {
                VariableState::Assigned { value: assigned_value, assignment_level, .. } => {
                    if assigned_value == l.sign {
                        (0, assignment_level as i64)
                    } else {
                        (2, -(assignment_level as i64))
                    }
                }
                VariableState::Unassigned { .. } => (1, 0),
            });
            assert!(variable_manager.is_true(literals[0]) || !variable_manager.is_assigned(literals[0]));
            // 先頭の 2 つを監視リテラルに
            for (k, literal) in literals.iter().enumerate().take(2) {
                self.watched_infos[literal.index][1 - (literal.sign as usize)]
                    .push(WatchedBy { clause_index: clause_index, watching_position: k });
            }

            if variable_manager.is_false(literals[1]) {
                // 末尾の監視リテラルに偽が割り当てられている場合には未割り当ての監視リテラルに真を割り当て
                if !variable_manager.is_assigned(literals[0]) {
                    tentative_assigned_variable_queue.insert(
                        literals[0].index,
                        literals[0].sign,
                        Reason::Propagation {
                            clause_index: clause_index,
                            assignment_level_at_propagated: variable_manager.current_assignment_level(),
                        },
                    );
                }
            }
        }
        //
        let plbd;
        if is_learnt {
            plbd = self.calculate_plbd.calculate(variable_manager, &literals);
            debug_assert!(literals.len() <= 1 || plbd >= 2);
            self.number_of_learnt_clauses += 1;
            self.plbd_ammount += plbd;

            self.current_plbd_deque.push_back(plbd);
            self.current_plbd_ammount += plbd;
            if self.current_plbd_deque.len() > 50 {
                let forgetting_plbd = self.current_plbd_deque.pop_front().unwrap();
                self.current_plbd_ammount -= forgetting_plbd;
            }
        } else {
            plbd = 0;
        }
        // 節を追加
        self.clause_infos.push(Clause {
            literals: literals,
            is_learnt: is_learnt,
            plbd,
            is_deleted: false,
            activity: self.activity_increase_value,
        });
    }

    pub fn inform_assignment(
        &mut self,
        variable_manager: &VariableManager,
        assigned_variable_index: usize,
        tentative_assigned_variable_queue: &mut TentativeAssignedVariableQueue,
    ) {
        let VariableState::Assigned { value: assigned_value, .. } = variable_manager.get_state(assigned_variable_index)
        else {
            unreachable!();
        };
        // NOTE: 移動するなら以降の処理を ClauseManager に
        let mut k = 0usize;
        // variable_index への value の割当を監視している節を走査
        // MEMO: self.watched_infos[variable_index][value as usize].len() が長くならない(領域の再確保が行われない)仮定を使えばもう少し速くできるんだろうか？
        'loop_watching_clause: while k < self.watched_infos[assigned_variable_index][assigned_value as usize].len() {
            self.check_count += 1;
            let WatchedBy { clause_index, watching_position } =
                self.watched_infos[assigned_variable_index][assigned_value as usize][k];
            // println!("c{}", clause_index);
            debug_assert!(watching_position < 2);
            let clause = &mut self.clause_infos[clause_index];
            let watched_literal = clause.literals[watching_position];
            debug_assert!(watched_literal.index == assigned_variable_index);
            debug_assert!(watched_literal.sign == !assigned_value);
            let another_watched_literal = clause.literals[1 - watching_position];
            debug_assert!(!variable_manager.is_false(another_watched_literal)); // 既に false が割り当てられていることはないはず
            if variable_manager.is_true(another_watched_literal) {
                // もう一方の監視リテラルに真が割り当てられており既に充足されているのでなにもしない
                self.skip_count += 1;
            } else {
                // 監視対象ではないリテラルを走査
                for (l, literal) in clause.literals.iter().enumerate().skip(2) {
                    if !variable_manager.is_false(*literal) {
                        // 真が割り当てられているまたは未割り当てのリテラルを発見した場合
                        // 元の監視リテラルの監視を解除
                        self.watched_infos[watched_literal.index][!watched_literal.sign as usize].swap_remove(k);
                        // 発見したリテラルを監視
                        self.watched_infos[literal.index][!literal.sign as usize]
                            .push(WatchedBy { clause_index: clause_index, watching_position });
                        // 発見したリテラルを監視位置に移動
                        clause.literals.swap(watching_position, l);
                        // 次の節へ
                        continue 'loop_watching_clause;
                    }
                }
                // 真が割り当てられているまたは未割り当てのリテラルが見つからなかった場合には，もう一方の監視リテラルに真を割り当て
                self.propagation_count += 1;
                tentative_assigned_variable_queue.insert(
                    another_watched_literal.index,
                    another_watched_literal.sign,
                    Reason::Propagation {
                        clause_index: clause_index,
                        assignment_level_at_propagated: variable_manager.current_assignment_level(),
                    },
                );
                if clause.is_learnt {
                    // pseudo_lbd を更新
                    let pseudo_lbd = self.calculate_plbd.calculate(variable_manager, &clause.literals);
                    if pseudo_lbd < clause.plbd {
                        self.plbd_ammount -= clause.plbd - pseudo_lbd;
                        clause.plbd = pseudo_lbd;
                    }
                }
            }
            k += 1;
        }
    }

    pub fn explain(&mut self, variable_index: usize, value: bool, reason: Reason, clause: &mut Vec<Literal>) {
        assert!(matches!(reason, Reason::Propagation { .. }));
        let Reason::Propagation { clause_index, .. } = reason else {
            unreachable!();
        };
        assert!(self.clause_infos[clause_index].literals.iter().any(|l| l.index == variable_index && l.sign == value));
        self.clause_infos[clause_index].activity += self.activity_increase_value;
        clause.clone_from(&self.clause_infos[clause_index].literals);
    }

    pub fn advance_time(&mut self) {
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
        if self.number_of_learnt_clauses == 0 {
            false
        } else {
            let pseudo_lbd_average = self.plbd_ammount as f64 / self.number_of_learnt_clauses as f64;
            let current_pseudo_lbd_average = self.current_plbd_ammount as f64 / 50.0;
            self.number_of_learnt_clauses > 10000 + 1000 * (self.clause_reduction_count + 1)
                || current_pseudo_lbd_average * 0.9 > pseudo_lbd_average
        }
    }

    pub fn restart(&mut self, variable_manager: &VariableManager) {
        assert!(variable_manager.current_decision_level() == 0);
        eprintln!(
            "pldb_average={} current_pldb_average={}",
            self.plbd_ammount as f64 / self.number_of_learnt_clauses as f64,
            self.current_plbd_ammount as f64 / 50.0
        );
        self.current_plbd_ammount = 0;
        self.current_plbd_deque.clear();
        if self.number_of_learnt_clauses > 10000 + 1000 * self.clause_reduction_count {
            self.clause_reduction_count += 1;
            let mut clause_priority_order = Vec::from_iter(
                (0..self.clause_infos.len())
                    .filter(|i| self.clause_infos[*i].is_learnt && !self.clause_infos[*i].is_deleted),
            );
            debug_assert!(clause_priority_order.len() == self.number_of_learnt_clauses);
            // 削除の優先度の高い順にソート
            clause_priority_order.sort_unstable_by(|l, r| {
                let lhs = (3 - self.clause_infos[*l].plbd.min(3), self.clause_infos[*l].activity);
                let rhs = (3 - self.clause_infos[*r].plbd.min(3), self.clause_infos[*r].activity);
                lhs.partial_cmp(&rhs).unwrap()
            });
            // とりあえず半分ぐらいを削除
            for clause_index in clause_priority_order.iter().take(clause_priority_order.len() / 2) {
                self.clause_infos[*clause_index].is_deleted = true;
            }
            // 削除された節の監視を削除
            for variable_index in 0..variable_manager.number_of_variables() {
                for value in [0, 1] {
                    let list = &mut self.watched_infos[variable_index][value];
                    let mut k: usize = 0;
                    while k < list.len() {
                        if self.clause_infos[list[k].clause_index].is_deleted {
                            list.swap_remove(k);
                        } else {
                            k += 1;
                        }
                    }
                }
            }
            // 全体の PLDB を再計算
            self.number_of_learnt_clauses = 0;
            self.plbd_ammount = 0;
            for clause in self.clause_infos.iter() {
                if clause.is_learnt && !clause.is_deleted {
                    self.number_of_learnt_clauses += 1;
                    self.plbd_ammount += clause.plbd;
                }
            }
            eprintln!("reduce learnt clauses {} -> {}", clause_priority_order.len(), self.number_of_learnt_clauses);
        }
    }

    pub fn summary(&self) -> (usize, usize, usize) {
        (self.check_count, self.skip_count, self.propagation_count)
    }
}
