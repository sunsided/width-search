//! Fixed seeds of small random SAS+ tasks, checked against the perfect
//! heuristic `h*`.

use sas_plus::random::{RandomTaskParams, random_task};
use width_search::{bfws, iterated_width, iw, sas::SasProblem};

/// Small tasks with unit costs, so `h*` is cheap and plan length is cost.
const PARAMS: RandomTaskParams = RandomTaskParams {
    num_vars: 4,
    max_domain: 3,
    num_ops: 10,
    max_cost: 1,
    max_conditions: 0,
    num_derived: 0,
};

/// IW(k) with k = #variables never prunes a new state (its full fact set is a
/// new k-tuple), so it is complete and, on unit costs, optimal like
/// breadth-first search.
#[test]
fn iw_with_full_width_is_complete_and_optimal() {
    let mut solved = 0;
    for seed in 0..300 {
        let task = random_task(seed, PARAMS);
        let h_star = task.perfect_heuristic()(&task.initial);
        let r = iw(&SasProblem::new(&task), task.num_variables());
        match (h_star, r.plan) {
            (Some(opt), Some(plan)) => {
                assert_eq!(task.validate_plan(&plan), Some(opt), "seed {seed}");
                solved += 1;
            }
            (None, None) => {}
            (h, p) => panic!("seed {seed}: h* = {h:?}, plan = {p:?}"),
        }
    }
    assert!(
        solved > 50,
        "generator produces too few solvable tasks ({solved})"
    );
}

/// Both searches find a plan exactly when `h*` says the task is solvable,
/// and BFWS plans validate.
#[test]
fn iterated_width_and_bfws_return_valid_plans() {
    for seed in 0..300 {
        let task = random_task(seed, PARAMS);
        let problem = SasProblem::new(&task);
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let (_, r) = iterated_width(&problem, task.num_variables());
        assert_eq!(r.plan.is_some(), solvable, "IW seed {seed}");
        let r = bfws(&problem, 2);
        assert_eq!(r.plan.is_some(), solvable, "BFWS seed {seed}");
        if let Some(plan) = r.plan {
            assert!(task.validate_plan(&plan).is_some(), "BFWS seed {seed}");
        }
    }
}

/// BFWS rejects width 0 even when the initial state is already a goal and no
/// novelty table is ever built.
#[test]
#[should_panic(expected = "width bound must be at least 1")]
fn bfws_rejects_width_zero_on_a_goal_initial_state() {
    let mut task = random_task(0, PARAMS);
    task.goal.clear();
    let problem = SasProblem::new(&task);

    let _ = bfws(&problem, 0);
}
