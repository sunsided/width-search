//! Width-based search on real Fast Downward translator output.

use sas_plus::fdr::SasFile;
use width_search::{bfws, iterated_width, sas::SasProblem};

/// Parses `tests/fixtures/translated/<name>/output.sas`.
fn load(name: &str) -> SasFile {
    let path = format!(
        "{}/tests/fixtures/translated/{name}/output.sas",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap().parse().unwrap()
}

/// BFWS(2) returns a valid plan on each of the larger translated tasks.
#[test]
fn bfws_solves_the_larger_benchmarks() {
    for name in ["bw-large-a", "logistics-e", "gripper-4", "sussman-anomaly"] {
        let file = load(name);
        let task = file.to_task();
        let r = bfws(&SasProblem::new(&task), 2);
        let plan = r.plan.unwrap_or_else(|| panic!("{name}: no plan"));
        let cost = task
            .validate_plan(&plan)
            .unwrap_or_else(|| panic!("{name}: invalid plan"));
        println!("{name}: BFWS plan of cost {cost}, {} expanded", r.expanded);
    }
}

/// gripper-2's goal is the single atom at(ball1, roomb). Its width is 2, not 1:
/// at-robby(roomb) and carry(ball1, left) are each first reached in separate
/// states, so the state holding both (needed for the drop) has no new atom,
/// only a new pair.
#[test]
fn single_atom_goal_has_width_2() {
    let task = load("gripper-2").to_task();
    let (width, r) = iterated_width(&SasProblem::new(&task), 2);
    assert_eq!(width, Some(2));
    assert_eq!(task.validate_plan(&r.plan.unwrap()), Some(3));
}
