use crate::finite_collections::Array;

use super::analyze::Analyze;
use super::clause_theory::ClauseTheory;
use super::types::{Literal, Reason, VariableSize};
use super::variables::{Variable, Variables};

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
    variables: Variables,
    clause_theory: ClauseTheory,
    analyze: Analyze,
    conflict_count: usize,
    restart_count: usize,
}

impl SATSolver {
    #[inline(never)]
    pub fn new() -> Self {
        SATSolver {
            variables: Variables::new(50.0),
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
        self.variables.redimension(self.variables.dimension() + additional);
        self.clause_theory.expand(additional);
    }

    #[inline(never)]
    pub fn add_clause(&mut self, literals: &Array<VariableSize, Literal>) {
        // println!("@add_clause");
        // 必要に応じて変数の次元を拡張
        let required_variable_dimension = literals.iter().map(|l| l.index + 1).max().unwrap_or(0);
        if required_variable_dimension > self.variables.dimension() {
            self.expand_variables(required_variable_dimension - self.variables.dimension());
        }

        self.clause_theory.add_clause(literals.clone(), false, &mut self.variables);
    }

    #[inline(never)]
    pub fn solve(&mut self) -> SATSolverResult {
        let search_result = self.search();
        match search_result {
            SearchResult::Satisfiable => {
                let mut solution = Array::default();
                for variable_index in 0..self.variables.dimension() {
                    if let Variable::Assigned(assigned_variable) = self.variables.get(variable_index) {
                        solution.push(*assigned_variable.value());
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
                if self.variables.current_decision_level() == 0 {
                    return SearchResult::Unsatisfiable;
                }
                // analyze
                let (backjump_decision_level, learnt_clause) =
                    self.analyze.analyze(variable_index, reasons, &mut self.variables, &mut self.clause_theory);
                // 長さ 0 の学習節が得られたら充足不可能
                if learnt_clause.len() == 0 {
                    return SearchResult::Unsatisfiable;
                }
                // 伝播可能な決定レベルまでバックジャンプ
                self.backjump(backjump_decision_level);
                // 学習節を追加
                self.clause_theory.add_clause(learnt_clause, true, &mut self.variables);
                // 時刻を 1 つ進める(内部でアクティビティの指数平滑化を行っているため)
                self.variables.advance_time();
                self.clause_theory.advance_time();
            } else if self.variables.number_of_assigned_variables() == self.variables.dimension() {
                // 未割り当ての変数がなくなれば充足可能
                return SearchResult::Satisfiable;
            } else if self.clause_theory.is_request_restart() {
                // 条件を満たしたらリスタート
                if self.variables.current_decision_level() != 0 {
                    self.backjump(0);
                }
                self.clause_theory.restart(&self.variables);
                eprintln!(
                    "restart_count={} conflict_count={} fixed={}",
                    self.restart_count,
                    self.conflict_count,
                    self.variables.number_of_assigned_variables(),
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
        debug_assert!(self.variables.first_conflicting_variable().is_none());
        debug_assert!(self.variables.first_tentatively_assigned_variable().is_none());
        let unassigned_variable = self.variables.first_unassigned_variable().unwrap();
        let last_assigned_value = *unassigned_variable.last_assigned_value();
        unassigned_variable.tentatively_assign(last_assigned_value, Reason::Decision);
    }

    #[inline(never)]
    fn backjump(&mut self, backjump_decision_level: VariableSize) {
        // println!("@backjump");
        // println!("{} to {}", self.variable_manager.current_decision_level(), backjump_decision_level);
        assert!(backjump_decision_level < self.variables.current_decision_level());

        self.variables.cancel_tentative_assignment();
        while self.variables.current_decision_level() > backjump_decision_level {
            self.variables.unassign();
        }
    }

    #[inline(never)]
    fn propagate(&mut self) -> PropagationResult {
        // println!("@propagate");
        loop {
            if let Some(conflicting_variable) = self.variables.first_conflicting_variable() {
                // 矛盾している変数が存在する場合
                // conflicting_assigned_variable への割当を行った節ののアクティビティを増大
                for reason in conflicting_variable.reasons().iter() {
                    if let Reason::Propagation { clause_index, .. } = reason {
                        self.clause_theory.increase_activity(*clause_index);
                    }
                }
                // Conflict を原因を返す
                return PropagationResult::Conflict {
                    variable_index: conflicting_variable.index(),
                    reasons: *conflicting_variable.reasons(),
                };
            } else if let Some(tentatively_assigned_variable) = self.variables.first_tentatively_assigned_variable() {
                // 仮割当変数が存在する場合
                // tentatively_assigned_variable への割当を行った節ののアクティビティを増大
                if let Reason::Propagation { clause_index, .. } = tentatively_assigned_variable.reason() {
                    self.clause_theory.increase_activity(*clause_index);
                }
                // 本割り当て
                let assigned_variable = tentatively_assigned_variable.assign();
                // 割り当てを通知
                self.clause_theory.propagate(assigned_variable.index(), &mut self.variables);
            } else {
                // 矛盾・仮割当状態の変数が存在しなければ Consistent を返す
                return PropagationResult::Consistent;
            }
        }
    }
}
