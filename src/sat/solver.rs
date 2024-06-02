use crate::finite_collections::Array;

use super::analyze::Analyze;
use super::clause_theory::ClauseTheory;
use super::tentative_assigned_variable_queue::TentativeAssignedVariableQueue;
use super::types::{Literal, VariableSize};
use super::unassigned_variable_queue::UnassignedVariableQueue;
use super::variable_manager::{Reason, VariableManager, VariableState};

enum SearchResult {
    Satisfiable,
    Unsatisfiable,
    // Undefined,
}

enum PropagationResult {
    Consistent,
    Conflict { variable_index: VariableSize, reasons: [Reason; 2] },
}

pub enum SATSolverResult {
    Satisfiable { solution: Array<VariableSize, bool> },
    Unsatisfiable,
}

pub struct SATSolver {
    variable_manager: VariableManager,
    tentative_assigned_variable_queue: TentativeAssignedVariableQueue,
    unassigned_variable_queue: UnassignedVariableQueue,
    clause_theory: ClauseTheory,
    analyze: Analyze,
    conflict_count: usize,
    restart_count: usize,
}

impl SATSolver {
    #[inline(never)]
    pub fn new() -> Self {
        SATSolver {
            variable_manager: VariableManager::default(),
            tentative_assigned_variable_queue: TentativeAssignedVariableQueue::default(),
            unassigned_variable_queue: UnassignedVariableQueue::new(20.0),
            clause_theory: ClauseTheory::new(100000, 100, 1000),
            analyze: Analyze::default(),
            conflict_count: 0usize,
            restart_count: 0usize,
        }
    }

    #[inline(never)]
    pub fn expand_variables(&mut self, additional: VariableSize) {
        if additional == 0 {
            return;
        }
        let original = self.variable_manager.number_of_variables();
        self.variable_manager.expand(additional);
        self.tentative_assigned_variable_queue.reserve(additional);
        self.unassigned_variable_queue.reserve(additional);
        // 追加した変数を未割り当て変数のキューに挿入
        for variable_index in original..self.variable_manager.number_of_variables() {
            self.unassigned_variable_queue.push(variable_index);
        }
        self.clause_theory.expand(additional);
        assert!(self.tentative_assigned_variable_queue.capacity() == self.variable_manager.number_of_variables());
        assert!(self.unassigned_variable_queue.capacity() == self.variable_manager.number_of_variables());
    }

    #[inline(never)]
    pub fn add_clause(&mut self, literals: &Array<VariableSize, Literal>) {
        // println!("@add_clause");
        // 必要に応じて変数の次元を拡張
        let required_variable_dimension = literals.iter().map(|l| l.index + 1).max().unwrap_or(0);
        if required_variable_dimension > self.variable_manager.number_of_variables() {
            self.expand_variables(required_variable_dimension - self.variable_manager.number_of_variables());
        }

        self.clause_theory.add_clause(
            &self.variable_manager,
            literals.clone(),
            false,
            &mut self.tentative_assigned_variable_queue,
            &self.unassigned_variable_queue,
        );
    }

    #[inline(never)]
    pub fn solve(&mut self) -> SATSolverResult {
        let search_result = self.search();
        match search_result {
            SearchResult::Satisfiable => {
                let mut solution = Array::default();
                for variable_index in 0..self.variable_manager.number_of_variables() {
                    if let VariableState::Assigned { value, .. } = self.variable_manager.get_state(variable_index) {
                        solution.push(value);
                    } else {
                        unreachable!();
                    }
                }
                return SATSolverResult::Satisfiable { solution: solution };
            }
            SearchResult::Unsatisfiable => {
                return SATSolverResult::Unsatisfiable;
            } // SearchResult::Undefined => {
              //     unreachable!();
              // }
        }
    }

    #[inline(never)]
    fn search(&mut self) -> SearchResult {
        loop {
            let propagation_result = self.propagate();
            if let PropagationResult::Conflict { variable_index, reasons } = propagation_result {
                // 矛盾を検知した場合
                self.conflict_count += 1;
                // 決定レベル 0 での矛盾であれば充足不可能
                if self.variable_manager.current_decision_level() == 0 {
                    return SearchResult::Unsatisfiable;
                }
                // analyze
                let (backjump_decision_level, learnt_clause) = self.analyze.analyze(
                    &self.variable_manager,
                    &mut self.clause_theory,
                    variable_index,
                    reasons,
                    &mut self.unassigned_variable_queue,
                );
                // 長さ 0 の学習節が得られたら充足不可能
                if learnt_clause.len() == 0 {
                    return SearchResult::Unsatisfiable;
                }
                // 伝播可能な決定レベルまでバックジャンプ
                self.backjump(backjump_decision_level);
                // 学習節を追加
                self.clause_theory.add_clause(
                    &self.variable_manager,
                    learnt_clause,
                    true,
                    &mut self.tentative_assigned_variable_queue,
                    &self.unassigned_variable_queue,
                );
                // 時刻を 1 つ進める(内部でアクティビティの指数平滑化を行っているため)
                self.unassigned_variable_queue.advance_time();
                self.clause_theory.advance_time();
            } else if self.variable_manager.number_of_unassigned_variables() == 0 {
                // 未割り当ての変数がなくなれば充足可能
                return SearchResult::Satisfiable;
            } else if self.clause_theory.is_request_restart() {
                // 条件を満たしたらリスタート
                if self.variable_manager.current_decision_level() != 0 {
                    self.backjump(0);
                }
                self.clause_theory.restart(&self.variable_manager);
                eprintln!(
                    "restart_count={} conflict_count={} fixed={}",
                    self.restart_count,
                    self.conflict_count,
                    self.variable_manager.number_of_assigned_variables(),
                );
                self.restart_count += 1;
            } else {
                // 決定変数を選択
                self.decide();
            }
        }
    }

    #[inline(never)]
    pub fn summary(&self) -> (usize, usize, usize, usize, usize, usize) {
        // TODO: 各種サマリを返せるようにしたい & 計算途中にコールバック関数でも返せるようにしたい
        let s = self.clause_theory.summary();
        (s.0, s.1, s.2, s.3, self.conflict_count, self.restart_count)
    }

    #[inline(never)]
    fn decide(&mut self) {
        // println!("@decide");
        assert!(self.variable_manager.number_of_assigned_variables() < self.variable_manager.number_of_variables());
        assert!(self.tentative_assigned_variable_queue.is_empty());
        loop {
            assert!(!self.unassigned_variable_queue.is_empty());
            let variable_index = self.unassigned_variable_queue.pop().unwrap();
            match self.variable_manager.get_state(variable_index) {
                VariableState::Assigned { .. } => {
                    // 伝播によって既に値が割り当てられた変数が含まれていることがあるので，それらを無視する
                    continue;
                }
                VariableState::Unassigned { last_assigned_value } => {
                    // println!("x{}={}", variable_index, last_assigned_value);
                    // 前に割り当てられていた値を割り当て
                    self.tentative_assigned_variable_queue.push(variable_index, last_assigned_value, Reason::Decision);
                    break;
                }
            }
        }
        assert!(!self.tentative_assigned_variable_queue.is_empty());
    }

    #[inline(never)]
    fn backjump(&mut self, backjump_decision_level: VariableSize) {
        // println!("@backjump");
        // println!("{} to {}", self.variable_manager.current_decision_level(), backjump_decision_level);
        assert!(backjump_decision_level < self.variable_manager.current_decision_level());
        self.tentative_assigned_variable_queue.clear();
        while self.variable_manager.current_decision_level() > backjump_decision_level {
            let unassigned_variable_index = self.variable_manager.unassign();
            self.unassigned_variable_queue.push(unassigned_variable_index);
        }
    }

    #[inline(never)]
    fn propagate(&mut self) -> PropagationResult {
        // println!("@propagate");
        while !self.tentative_assigned_variable_queue.is_empty() {
            if self.tentative_assigned_variable_queue.has_conflict() {
                // 矛盾が発生していれば矛盾している変数を 1 つ選んで返す
                let (variable_index, reasons) =
                    self.tentative_assigned_variable_queue.pop_conflicting_variable().unwrap();
                for reason in reasons.iter() {
                    if let Reason::Propagation { clause_index, .. } = *reason {
                        self.clause_theory.increase_activity(clause_index);
                    }
                }
                return PropagationResult::Conflict { variable_index: variable_index, reasons: reasons };
            }
            // 伝播によって仮割り当てされた変数のうち最も優先度の高いものを取り出す
            let (variable_index, value, reason) =
                self.tentative_assigned_variable_queue.pop_consistent_variable().unwrap();
            // 本割り当て
            self.variable_manager.assign(variable_index, value, reason);
            //
            if let Reason::Propagation { clause_index, .. } = reason {
                self.clause_theory.increase_activity(clause_index);
            }
            // 割り当てを通知
            self.clause_theory.propagate(
                &self.variable_manager,
                variable_index,
                &mut self.tentative_assigned_variable_queue,
                &self.unassigned_variable_queue,
            );
        }

        PropagationResult::Consistent
    }
}
