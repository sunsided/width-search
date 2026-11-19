use sas_plus::random::{RandomTaskParams, random_task};
use width_search::{bfws, iterated_width, iw, sas::SasProblem};

const PARAMS: RandomTaskParams = RandomTaskParams {
    num_vars: 4,
    max_domain: 3,
    num_ops: 10,
    max_cost: 1,
};

/// IW(k) with k = #variables never prunes a new state (its full fact set is a
/// new k-tuple), so it is complete and, on unit costs, optimal like BrFS.
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
