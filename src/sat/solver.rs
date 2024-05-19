use crate::finite_collections::finite_heaped_map;
use crate::finite_collections::FiniteHeapedMap;

use super::clause_theory::ClauseTheory;
use super::tentative_assigned_variable_queue::TentativeAssignedVariableQueue;
use super::types::Literal;
use super::unassigned_variable_queue::UnassignedVariableQueue;
use super::variable_manager::Reason;
use super::variable_manager::VariableManager;
use super::variable_manager::VariableState;

enum SearchResult {
    Satisfiable,
    Unsatisfiable,
    Undefined,
}

enum PropagationResult {
    Consistent,
    Conflict { variable_index: usize, reasons: [Reason; 2] },
}

pub enum SATSolverResult {
    Satisfiable { solution: Vec<bool> },
    Unsatisfiable,
}

pub struct SATSolver {
    variable_manager: VariableManager,
    tentative_assigned_variable_queue: TentativeAssignedVariableQueue,
    unassigned_variable_queue: UnassignedVariableQueue,
    clause_theory: ClauseTheory,
    // TODO これが必要になるなら analyze も別の構造体とした方がよいか
    literal_buffer: Vec<Literal>,
    analyzer_buffer: FiniteHeapedMap<AnalyzerBufferValue, AnalyzerBufferComparator>,
    conflict_count: usize,
    restart_count: usize,
}

impl SATSolver {
    #[inline(never)]
    pub fn new() -> Self {
        SATSolver {
            variable_manager: VariableManager::default(),
            tentative_assigned_variable_queue: TentativeAssignedVariableQueue::default(),
            unassigned_variable_queue: UnassignedVariableQueue::new(1e5),
            clause_theory: ClauseTheory::new(),
            literal_buffer: Vec::default(),
            analyzer_buffer: FiniteHeapedMap::default(),
            conflict_count: 0usize,
            restart_count: 0usize,
        }
    }

    #[inline(never)]
    pub fn expand_variables(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }
        let original = self.variable_manager.number_of_variables();
        self.variable_manager.expand(additional);
        self.tentative_assigned_variable_queue.reserve(additional);
        self.unassigned_variable_queue.reserve(additional);
        // 追加した変数を未割り当て変数のキューに挿入
        for variable_index in original..self.variable_manager.number_of_variables() {
            self.unassigned_variable_queue.insert(variable_index);
        }
        self.clause_theory.expand(additional);
        assert!(self.tentative_assigned_variable_queue.capacity() == self.variable_manager.number_of_variables());
        assert!(self.unassigned_variable_queue.capacity() == self.variable_manager.number_of_variables());
    }

    #[inline(never)]
    pub fn add_clause(&mut self, literals: Vec<Literal>) {
        // println!("@add_clause");
        // 必要に応じて変数の次元を拡張
        let required_variable_dimension = literals.iter().map(|l| l.index + 1).max().unwrap_or(0);
        if required_variable_dimension > self.variable_manager.number_of_variables() {
            self.expand_variables(required_variable_dimension - self.variable_manager.number_of_variables());
        }

        self.clause_theory.add_clause(
            &self.variable_manager,
            literals,
            false,
            &mut self.tentative_assigned_variable_queue,
        );
    }

    #[inline(never)]
    pub fn solve(&mut self) -> SATSolverResult {
        loop {
            let search_result = self.search(100 * Self::luby(self.restart_count) + self.restart_count);
            match search_result {
                SearchResult::Satisfiable => {
                    let mut solution = Vec::default();
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
                }
                SearchResult::Undefined => {
                    self.restart_count += 1;
                    // TODO 学習節の削減
                }
            }
        }
    }

    #[inline(never)]
    fn search(&mut self, conflict_count_limit: usize) -> SearchResult {
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
                let (backjump_decision_level, learnt_clause) = self.analyze(variable_index, reasons);
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
                );
                // 時刻を 1 つ進める(内部でアクティビティの指数平滑化を行っているため)
                self.unassigned_variable_queue.advance_time();
            } else if self.variable_manager.number_of_unassigned_variables() == 0 {
                // 未割り当ての変数がなくなれば充足可能
                return SearchResult::Satisfiable;
            } else if self.conflict_count >= conflict_count_limit {
                // 矛盾回数が閾値に達したらリスタート
                if self.variable_manager.current_decision_level() != 0 {
                    self.backjump(0);
                }
                return SearchResult::Undefined;
            } else {
                // 決定変数を選択
                self.decide();
            }
        }
    }

    #[inline(never)]
    pub fn summary(&self) -> usize {
        // TODO: 各種サマリを返せるようにしたい & 計算途中にコールバック関数でも返せるようにしたい
        self.conflict_count
    }

    #[inline(never)]
    fn luby(i: usize) -> usize {
        let mut j = i + 1;
        let mut p = 2usize;
        while j > p - 1 {
            p *= 2;
        }
        while j != p - 1 {
            j = j - p / 2 + 1;
            while p / 2 > j {
                p /= 2;
            }
        }
        p / 2
    }

    #[inline(never)]
    fn decide(&mut self) {
        // println!("@decide");
        assert!(self.variable_manager.number_of_assigned_variables() < self.variable_manager.number_of_variables());
        assert!(self.tentative_assigned_variable_queue.is_empty());
        loop {
            assert!(!self.unassigned_variable_queue.is_empty());
            let variable_index = self.unassigned_variable_queue.pop_first().unwrap();
            match self.variable_manager.get_state(variable_index) {
                VariableState::Assigned { .. } => {
                    // 伝播によって既に値が割り当てられた変数が含まれていることがあるので，それらを無視する
                    continue;
                }
                VariableState::Unassigned { last_assigned_value } => {
                    // println!("x{}={}", variable_index, last_assigned_value);
                    // 前に割り当てられていた値を割り当て
                    self.tentative_assigned_variable_queue.insert(
                        variable_index,
                        last_assigned_value,
                        Reason::Decision,
                    );
                    break;
                }
            }
        }
        assert!(!self.tentative_assigned_variable_queue.is_empty());
    }

    #[inline(never)]
    fn backjump(&mut self, backjump_decision_level: usize) {
        // println!("@backjump");
        // println!("{} to {}", self.variable_manager.current_decision_level(), backjump_decision_level);
        assert!(backjump_decision_level < self.variable_manager.current_decision_level());
        self.tentative_assigned_variable_queue.clear();
        while self.variable_manager.current_decision_level() > backjump_decision_level {
            let unassigned_variable_index = self.variable_manager.unassign();
            self.unassigned_variable_queue.insert(unassigned_variable_index);
        }
    }

    #[inline(never)]
    fn propagate(&mut self) -> PropagationResult {
        // println!("@propagate");
        while !self.tentative_assigned_variable_queue.is_empty() {
            if self.tentative_assigned_variable_queue.has_conflict() {
                // 矛盾が発生していれば矛盾している変数を 1 つ選んで返す
                let (variable_index, reasons) =
                    self.tentative_assigned_variable_queue.iter_conflicting_variables().next().unwrap();
                return PropagationResult::Conflict { variable_index: *variable_index, reasons: reasons.clone() };
            }
            // 伝播によって仮割り当てされた変数のうち最も優先度の高いものを取り出す
            let (variable_index, value, reason) =
                self.tentative_assigned_variable_queue.pop_first_consistent_variable().unwrap();
            // println!("by x{}={}", variable_index, value);
            // println!("watching_clauses={}", self.watched_infos[variable_index][value as usize].len());
            // 本割り当て
            self.variable_manager.assign(variable_index, value, reason);
            // 割り当てを通知
            self.clause_theory.inform_assignment(
                &self.variable_manager,
                variable_index,
                &mut self.tentative_assigned_variable_queue,
            );
        }

        PropagationResult::Consistent
    }

    #[inline(never)]
    fn resolve(&mut self, variable_index: usize, value: bool, reason: Reason) {
        // MEMO: このあたりはもう少しマシな設計がある気がする
        // 割り当てを説明する節を取得
        self.literal_buffer.clear();
        self.clause_theory.explain(variable_index, value, reason, &mut self.literal_buffer);
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
                if let VariableState::Assigned { decision_level, assignment_level, reason, .. } =
                    self.variable_manager.get_state(literal.index)
                {
                    if decision_level != 0 {
                        self.analyzer_buffer.insert(
                            literal.index,
                            AnalyzerBufferValue {
                                sign: literal.sign,
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

    #[inline(never)]
    fn analyze(&mut self, conflicting_variable_index: usize, reasons: [Reason; 2]) -> (usize, Vec<Literal>) {
        // println!("@analyze");
        self.analyzer_buffer.clear();
        if self.analyzer_buffer.len() < self.variable_manager.number_of_variables() {
            self.analyzer_buffer.reserve(self.variable_manager.number_of_variables() - self.analyzer_buffer.len());
        }
        // 矛盾が生じている変数のアクティビティを増大
        self.unassigned_variable_queue.increase_activity(conflicting_variable_index);
        // 矛盾している 2 つの節を融合
        for (reason, value) in reasons.iter().zip([false, true]) {
            self.resolve(conflicting_variable_index, value, *reason);
        }
        // バックジャンプ可能な節が獲られるまで融合を繰り返す
        loop {
            if self.analyzer_buffer.len() == 0 {
                // 節融合の結果が空になった場合には空の学習節を返す(Unsatisifiable)
                return (0, vec![]);
            }
            {
                // バックジャンプ可能かを判定
                // 二分ヒープの先頭 3 つの decision_level を取得
                // NOTE: 二分ヒープなので，最大要素と 2 番目に大きい要素を使用するなら，最初の 3 要素だけを見れば十分．
                let first_three_decision_levels =
                    Vec::from_iter(self.analyzer_buffer.iter().take(3).map(|item| item.1.decision_level));
                assert!(first_three_decision_levels.len() >= 1);
                // 最も大きい決定レベルは現在の決定レベルと同じであるはず
                assert!(first_three_decision_levels[0] == self.variable_manager.current_decision_level());
                // 2 番目に大きい決定レベルを取得
                let second_largest_decision_level = first_three_decision_levels[1..].iter().max().unwrap_or(&0);
                if *second_largest_decision_level < self.variable_manager.current_decision_level() {
                    // 2 番目に大きい決定レベルが現在の決定レベル未満であればバックジャンプ可能なので，現在の節を学習節として返す
                    let mut learnt_clause = Vec::default();
                    for (variable_index, buffer_value) in self.analyzer_buffer.iter() {
                        learnt_clause.push(Literal { index: *variable_index, sign: buffer_value.sign });
                        // 学習節に含まれる変数のアクティビティを増大
                        self.unassigned_variable_queue.increase_activity(*variable_index);
                    }
                    return (*second_largest_decision_level, learnt_clause);
                }
            }
            // 最大割り当てレベル(節融合による消去対象)の変数を選択
            let variable_index = self.analyzer_buffer.first_key_value().unwrap().0;
            let value = !self.analyzer_buffer.first_key_value().unwrap().1.sign;
            let reason = self.analyzer_buffer.first_key_value().unwrap().1.reason;
            // 消去対象の変数のアクティビティを増大
            self.unassigned_variable_queue.increase_activity(variable_index);
            // 節融合
            self.resolve(variable_index, value, reason);
        }
    }
}

struct AnalyzerBufferValue {
    sign: bool,
    decision_level: usize,
    assignment_level: usize,
    reason: Reason,
}

struct AnalyzerBufferComparator {}

impl finite_heaped_map::Comparator<AnalyzerBufferValue> for AnalyzerBufferComparator {
    #[inline(always)]
    fn compare(lhs: &(usize, AnalyzerBufferValue), rhs: &(usize, AnalyzerBufferValue)) -> std::cmp::Ordering {
        rhs.1.assignment_level.cmp(&lhs.1.assignment_level)
    }
}
