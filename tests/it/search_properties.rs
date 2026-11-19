//! Property tests over random SAS+ tasks: every search returns only valid
//! plans and none for unsolvable tasks, and both BFWS variants find one
//! exactly when the task is solvable.

use proptest::prelude::*;
use sas_plus::Task;
use sas_plus::random::{RandomTaskParams, random_task};
use width_search::{bfws, bfws_r, iterated_width, sas::SasProblem, siw};

/// A random task from a seed and small size parameters, so `h*` stays cheap.
/// Conditional effects and derived variables are drawn too, so the adapter
/// is checked on the full task model and not only on plain STRIPS-like tasks.
fn task() -> impl Strategy<Value = Task> {
    (
        any::<u64>(),
        1usize..=5,
        2u32..=4,
        1usize..=12,
        1u32..=3,
        0usize..=2,
        0usize..=2,
    )
        .prop_map(
            |(seed, num_vars, max_domain, num_ops, max_cost, max_conditions, num_derived)| {
                random_task(
                    seed,
                    RandomTaskParams {
                        num_vars,
                        max_domain,
                        num_ops,
                        max_cost,
                        max_conditions,
                        num_derived,
                    },
                )
            },
        )
}

proptest! {
    /// BFWS is complete on finite state spaces, and its plans are valid.
    #[test]
    fn bfws_plans_are_valid_and_complete(task in task(), k in 1usize..=2) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let plan = bfws(&SasProblem::new(&task), k).plan;

        prop_assert_eq!(plan.is_some(), solvable);
        if let Some(plan) = plan {
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// Iterated width may give up below the task's width, but any plan it
    /// returns is valid, and none is returned for an unsolvable task.
    #[test]
    fn iterated_width_plans_are_valid(task in task(), max_k in 1usize..=3) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let (width, result) = iterated_width(&SasProblem::new(&task), max_k);

        prop_assert_eq!(width.is_some(), result.plan.is_some());
        if let Some(plan) = result.plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// BFWS with `#r` is complete on finite state spaces too, and its plans
    /// are valid. Random tasks draw axioms and conditional effects, so this
    /// also exercises the adapter's relaxed plans.
    #[test]
    fn bfws_r_plans_are_valid_and_complete(task in task(), k in 1usize..=2) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let plan = bfws_r(&SasProblem::new(&task), k).plan;

        prop_assert_eq!(plan.is_some(), solvable);
        if let Some(plan) = plan {
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// SIW may give up, but any plan it returns is valid, and none is
    /// returned for an unsolvable task.
    #[test]
    fn siw_plans_are_valid(task in task(), max_k in 1usize..=2) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let plan = siw(&SasProblem::new(&task), max_k).plan;

        if let Some(plan) = plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }
}
