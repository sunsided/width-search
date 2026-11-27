//! Width-based search on real Fast Downward translator output.

use sas_plus::fdr::SasFile;
use width_search::{bfws, ihiw, iterated_width, iw, sas::SasProblem, siw, siw_plus};

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

/// SIW+ reaches every subgoal of the translated tasks at width 1, where
/// plain SIW needs width 2 on all but bw-large-a: partitioning novelty by
/// `#r` keeps the states that IW(1) prunes on the way to a subgoal.
#[test]
fn siw_plus_solves_the_benchmarks_at_width_1() {
    for name in [
        "gripper-2",
        "sussman-anomaly",
        "gripper-4",
        "bw-large-a",
        "logistics-e",
    ] {
        let task = load(name).to_task();
        let problem = SasProblem::new(&task);
        let plan = siw_plus(&problem, 1)
            .plan
            .unwrap_or_else(|| panic!("{name}: no plan"));

        assert!(task.validate_plan(&plan).is_some(), "{name}: invalid plan");
        assert_eq!(
            siw(&problem, 1).plan.is_some(),
            name == "bw-large-a",
            "{name}"
        );
    }
}

/// IHIW(1, 1) solves the translated tasks IW(2) solves: gripper-2 and
/// sussman-anomaly, where IW(1) fails, by promoting features found in its
/// pruned states. Like IW(2), it fails on the other three.
#[test]
fn ihiw_matches_iw2_coverage() {
    for name in ["gripper-2", "sussman-anomaly", "gripper-4"] {
        let task = load(name).to_task();
        let problem = SasProblem::new(&task);
        let iw2 = iw(&problem, 2).plan.is_some();
        for seed in 0..3 {
            let r = ihiw(&problem, 1, 1, seed);

            assert_eq!(r.plan.is_some(), iw2, "{name} seed {seed}");
            if let Some(plan) = r.plan {
                assert!(task.validate_plan(&plan).is_some(), "{name} seed {seed}");
            }
        }
    }
}
