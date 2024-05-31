use super::types::{ConstraintSize, Literal, VariableSize};
use crate::finite_collections::Array;

#[derive(Default)]
pub struct SATProblem {
    pub clauses: Array<ConstraintSize, Array<VariableSize, Literal>>,
}

fn parse_line(line: &str) -> Array<VariableSize, Literal> {
    let mut clause = Array::default();
    let mut found_zero = false;
    for field in line.split_whitespace() {
        assert!(!found_zero);
        let i = field.parse::<i64>().unwrap();
        if i == 0 {
            found_zero = true;
            continue;
        }
        clause.push(Literal { sign: i > 0, index: (i.abs() - 1) as VariableSize })
    }
    assert!(found_zero);
    clause
}

pub fn read_cnf<R>(reader: R) -> SATProblem
where
    R: std::io::BufRead,
{
    let mut problem = SATProblem::default();
    for read_result in reader.lines() {
        let line = read_result.unwrap();
        if line.is_empty() || line.starts_with('c') || line.starts_with('p') {
            continue;
        }
        problem.clauses.push(parse_line(&line))
    }
    problem
}
