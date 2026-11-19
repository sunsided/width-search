//! End to end: Fast Downward `output.sas` → `sas_plus::Task` → width-based search.

use sas_plus::fdr::SasFile;
use width_search::{bfws, iterated_width, sas::SasProblem};

/// Parses the hand-encoded gripper task (`tests/fixtures/gripper-unit/output.sas`).
fn gripper() -> SasFile {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/gripper-unit/output.sas"
    );
    std::fs::read_to_string(path).unwrap().parse().unwrap()
}

/// The conjunctive goal (both balls in room B) has width 3: IW(2) prunes too
/// much. Serialized IW (one goal atom at a time) is what keeps width low.
#[test]
fn iterated_width_solves_gripper_at_width_3() {
    let file = gripper();
    let task = file.to_task();
    let (width, r) = iterated_width(&SasProblem::new(&task), 3);
    let plan = r.plan.expect("gripper is solvable");
    assert_eq!(width, Some(3));
    let cost = task.validate_plan(&plan).expect("valid plan");
    assert!(cost >= 5, "cheaper than optimal: {cost}");
    println!("IW({}) plan, cost {cost} (optimal: 5)", width.unwrap());
    print!("{}", file.format_plan(&plan));
}

/// BFWS(2) returns a valid plan for gripper.
#[test]
fn bfws_solves_gripper() {
    let task = gripper().to_task();
    let plan = bfws(&SasProblem::new(&task), 2)
        .plan
        .expect("gripper is solvable");
    assert!(task.validate_plan(&plan).is_some());
}
