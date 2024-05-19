use super::types::Literal;
use super::variable_manager::VariableManager;
use super::variable_manager::VariableState;
use super::variable_manager::Reason;
use super::tentative_assigned_variable_queue::TentativeAssignedVariableQueue;


#[derive(Clone)]
struct WatchedBy
{
    clause_index: usize,
    watching_position: usize,
}

struct Clause
{
    literals: Vec<Literal>,
    is_learnt: bool,
}


pub struct ClauseTheory {
    watched_infos: Vec<[Vec<WatchedBy>; 2]>, // NOTE: Literal を添え字にしてアクセスできる配列を使いたい
    clause_infos: Vec<Clause>,
}

impl ClauseTheory {

    pub fn new() -> Self {
        ClauseTheory {
            watched_infos: Vec::default(),
            clause_infos: Vec::default(),
        }
    }

    pub fn expand(&mut self, additional: usize) {
        self.watched_infos.resize_with(
            self.watched_infos.len() + additional,
             || [vec![], vec![]]
        );
    }

    pub fn add_clause(
        &mut self,
        variable_manager: &VariableManager,
        mut literals: Vec<Literal>,
        is_learnt: bool,
        tentative_assigned_variable_queue: &mut TentativeAssignedVariableQueue
    ) {

        let clause_index = self.clause_infos.len();
        // TODO: あとで対応(すべてのリテラルに偽が割り当てられているケースはひとまず考えない)
        assert!(!literals.iter().all(|literal| variable_manager.is_false(*literal)));
        if literals.len() == 0 {
            assert!(false); // TODO: あとで対応(上の all での判定で除かれるはず)
        } else if literals.len() == 1 {
            if !variable_manager.is_assigned(literals[0]) {
                tentative_assigned_variable_queue.insert(
                    literals[0].index,
                    literals[0].sign,
                    Reason::Propagation {
                        clause_index: clause_index,
                        assignment_level_at_propagated: variable_manager.current_assignment_level()
                    }
                );
            }
        } else {
            /* 割当の状態に応じてリテラルをソート
            * 1. 真が割り当てられている -> 未割り当て -> 偽が割り当てられているの順
            * 2. 真が割り当てられているリテラル同士では割当レベルの昇順
            *    偽が割り当てられているリテラル同士では割当レベルの降順
            */
            literals.sort_by_cached_key(|l| {
                match variable_manager.get_state(l.index) {
                    VariableState::Assigned { value: assigned_value, assignment_level, ..} => {
                        if assigned_value == l.sign {
                            (0, assignment_level as i64)
                        } else {
                            (2, -(assignment_level as i64))
                        }
                    },
                    VariableState::Unassigned { .. } => {
                        (1, 0)
                    }
                }
            });
            assert!(variable_manager.is_true(literals[0]) || !variable_manager.is_assigned(literals[0]));
            // 先頭の 2 つを監視リテラルに
            for k in 0..2 {
                let literal = &literals[k];
                self.watched_infos[literal.index][1 - (literal.sign as usize)].push(WatchedBy{
                    clause_index: clause_index,
                    watching_position: k,
                });
            }
            
            if variable_manager.is_false(literals[1]) {
                // 末尾の監視リテラルに偽が割り当てられている場合には未割り当ての監視リテラルに真を割り当て
                if !variable_manager.is_assigned(literals[0]) {
                    tentative_assigned_variable_queue.insert(
                        literals[0].index,
                        literals[0].sign,
                        Reason::Propagation{
                            clause_index: clause_index,
                            assignment_level_at_propagated: variable_manager.current_assignment_level()
                        }
                    );
                }
            }
        }
       // 節を追加
        self.clause_infos.push(Clause{literals: literals, is_learnt: is_learnt});

    }

    pub fn inform_assignment(
        &mut self,
        variable_manager: &VariableManager,
        assigned_variable_index: usize,
        tentative_assigned_variable_queue: &mut TentativeAssignedVariableQueue
    ) {
        let VariableState::Assigned{value: assigned_value, ..} = variable_manager.get_state(assigned_variable_index) else { unreachable!();};
        // NOTE: 移動するなら以降の処理を ClauseManager に
        let mut k = 0usize;
        // variable_index への value の割当を監視している節を走査
        // MEMO: self.watched_infos[variable_index][value as usize].len() が長くならない(領域の再確保が行われない)仮定を使えばもう少し速くできるんだろうか？
        'loop_watching_clause: while k < self.watched_infos[assigned_variable_index][assigned_value as usize].len() {
            let WatchedBy {clause_index, watching_position} = self.watched_infos[assigned_variable_index][assigned_value as usize][k].clone();
            // println!("c{}", clause_index);
            assert!(watching_position < 2);
            let clause = &mut self.clause_infos[clause_index];
            let watched_literal = clause.literals[watching_position];
            assert!(watched_literal.index == assigned_variable_index);
            assert!(watched_literal.sign == !assigned_value);
            let another_watched_literal = clause.literals[1 - watching_position];
            assert!(!variable_manager.is_false(another_watched_literal)); // 既に false が割り当てられていることはないはず
            if variable_manager.is_true(another_watched_literal) {
                // もう一方の監視リテラルに真が割り当てられており既に充足されているのでなにもしない
            } else {
                // 監視対象ではないリテラルを走査
                for l in 2..clause.literals.len() {
                    let literal = clause.literals[l];
                    if !variable_manager.is_false(literal) {
                        // 真が割り当てられているまたは未割り当てのリテラルを発見した場合
                        // 発見したリテラルを監視位置に移動
                        clause.literals.swap(watching_position, l);
                        // 元の監視リテラルの監視を解除
                        self.watched_infos[watched_literal.index][!watched_literal.sign as usize].swap_remove(k);
                        // 発見したリテラルを監視
                        self.watched_infos[literal.index][!literal.sign as usize].push(
                            WatchedBy{
                                clause_index: clause_index,
                                watching_position,
                            }
                        );
                        // 次の節へ
                        continue 'loop_watching_clause;
                    }
                }
                // 真が割り当てられているまたは未割り当てのリテラルが見つからなかった場合には，もう一方の監視リテラルに真を割り当て
                tentative_assigned_variable_queue.insert(
                    another_watched_literal.index,
                    another_watched_literal.sign,
                    Reason::Propagation{
                        clause_index: clause_index,
                        assignment_level_at_propagated: variable_manager.current_assignment_level()
                    }
                );
            }
            k += 1;
        }

    }

    pub fn explain(&self, variable_index: usize, value: bool, reason: Reason, clause: &mut Vec<Literal>) {
        assert!(matches!(reason, Reason::Propagation { .. }));
        let Reason::Propagation { clause_index, ..} = reason else { unreachable!(); };
        assert!(self.clause_infos[clause_index].literals.iter().any(|l| l.index == variable_index && l.sign == value));
        clause.clone_from(&self.clause_infos[clause_index].literals);
    }

}