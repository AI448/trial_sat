mod finite_collections;
mod sat;

use sat::{SATSolver, SATSolverResult};


// fn print_sat_problem(problem: &SATProblem) {
//     for clause in problem.clauses.iter() {
//         let mut first = true;
//         for literal in clause.iter() {
//             if first {
//                 first = false;
//             } else {
//                 print!(" v ");
//             }
//             print!("{}x{}", if literal.sign { "" } else { "!" }, literal.index);
//         }
//         print!("\n");
//     }
// }


fn main() {


    let problem = sat::read_cnf(std::io::BufReader::new(std::io::stdin()));
    // print_sat_problem(&problem);

    let mut solver = SATSolver::new();
    for clause in problem.clauses.iter() {
        solver.add_clause(clause.clone());
    }
    let result = solver.solve();
    match result {
        SATSolverResult::Satisfiable { solution } => {
            // チェック
            for clause in problem.clauses.iter() {
                let mut is_satisfied = false;
                for literal in clause.iter() {
                    if solution[literal.index] == literal.sign {
                        is_satisfied = true;
                        break;
                    }
                }
                if !is_satisfied {
                    eprintln!("BAGUTTERU!");
                    return;
                }
            }
            println!("SATISFIABLE");
        },
        SATSolverResult::Unsatisfiable => {
            println!("UNSATISFIABLE");
        }
    }

}