//! Runs every planning search on a Fast Downward task and compares them.
//!
//! Give it the `output.sas` that Fast Downward's translator writes for a PDDL
//! domain and problem (`fast-downward.py --translate domain.pddl
//! problem.pddl`). Without an argument it uses a bundled gripper task with
//! four balls. The `sas-plus` feature provides the adapter, which supplies
//! the goal count and relaxed plans. Run:
//! `cargo run --release --features sas-plus --example fd_task [-- path/to/output.sas]`.

use std::time::Instant;

use sas_plus::Task;
use sas_plus::fdr::SasFile;
use width_search::sas::SasProblem;
use width_search::{SearchResult, bfws, bfws_r, ihiw, iterated_width, siw, siw_plus};

/// The task used when no path is given.
const DEFAULT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/translated/gripper-4/output.sas"
);

/// Runs `search`, checks its plan against the task, and prints one row.
fn run(task: &Task, name: &str, search: impl FnOnce() -> SearchResult<usize>) {
    let start = Instant::now();
    let r = search();
    let seconds = start.elapsed().as_secs_f64();
    let plan = match &r.plan {
        Some(plan) => {
            let cost = task
                .validate_plan(plan)
                .expect("every returned plan is valid");
            format!("cost {cost}")
        }
        None => "no plan".to_string(),
    };

    println!(
        "{name:<24} {plan:>10} {:>9} expanded {seconds:>8.3} s",
        r.expanded
    );
}

/// Loads the task and runs the searches.
///
/// # Panics
///
/// Panics if the file cannot be read or is not a valid `output.sas`.
fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT.to_string());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let file: SasFile = text.parse().unwrap_or_else(|e| panic!("{path}: {e:?}"));
    let task = file.to_task();
    let problem = SasProblem::new(&task);
    println!(
        "{path}\n{} variables, {} operators, {} goal facts\n",
        task.variables.len(),
        task.operators.len(),
        task.goal.len()
    );

    run(&task, "IW(1), IW(2)", || iterated_width(&problem, 2).1);
    run(&task, "SIW, max width 2", || siw(&problem, 2));
    run(&task, "SIW+, max width 1", || siw_plus(&problem, 1));
    run(&task, "IHIW(1, 1)", || ihiw(&problem, 1, 1, 0));
    run(&task, "BFWS, k=2", || bfws(&problem, 2));
    run(&task, "BFWS with #r, k=2", || bfws_r(&problem, 2));
}
