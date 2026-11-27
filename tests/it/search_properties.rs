//! Property tests over random SAS+ tasks: every search returns only valid
//! plans and none for unsolvable tasks, and both BFWS variants find one
//! exactly when the task is solvable.

use proptest::prelude::*;
use sas_plus::Task;
use sas_plus::random::{RandomTaskParams, random_task};
use width_search::{
    Condition, Effect, GoalCount, Rule, bfws, bfws_r, hiw, ihiw, iterated_width, iw, pi_iw,
    rollout_iw, sas::SasProblem, siw, siw_plus, siw_r,
};

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

    /// SIW+ may give up like SIW, but any plan it returns is valid, and
    /// none is returned for an unsolvable task.
    #[test]
    fn siw_plus_plans_are_valid(task in task(), max_k in 1usize..=2) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let plan = siw_plus(&SasProblem::new(&task), max_k).plan;

        if let Some(plan) = plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// Rollout IW terminates, returns only valid plans and none for an
    /// unsolvable task, and the same seed gives the same result.
    #[test]
    fn rollout_iw_plans_are_valid_and_seeded(task in task(), seed in any::<u64>()) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let problem = SasProblem::new(&task);
        let r = rollout_iw(&problem, seed);
        let again = rollout_iw(&problem, seed);

        prop_assert_eq!(&r.plan, &again.plan);
        prop_assert_eq!((r.expanded, r.generated), (again.expanded, again.generated));
        if let Some(plan) = r.plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// π-IW with a policy that hashes each successor to a weight in 0..=2,
    /// zeros included, terminates and returns only valid plans, none for an
    /// unsolvable task.
    #[test]
    fn pi_iw_plans_are_valid(task in task(), seed in any::<u64>()) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let policy = |_: &_, succ: &[(usize, _)], out: &mut Vec<f64>| {
            out.extend(succ.iter().map(|&(a, _)| f64::from(u8::try_from(a % 3).unwrap())));
        };
        let r = pi_iw(&SasProblem::new(&task), policy, seed);

        if let Some(plan) = r.plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// SIW-R with the goal-count sketch `{#g > 0} ↦ {#g↓}` is SIW: same
    /// plan, same node counts.
    #[test]
    fn siw_r_with_the_goal_count_sketch_is_siw(task in task(), max_k in 1usize..=2) {
        let problem = SasProblem::new(&task);
        let rules = [Rule {
            conditions: vec![Condition::Positive(0)],
            effects: vec![Effect::Decrease(0)],
        }];
        let goal_count = |s: &_, out: &mut Vec<u64>| {
            out.push(u64::try_from(problem.unsatisfied_goals(s)).unwrap());
        };
        let r = siw_r(&problem, &rules, goal_count, max_k);
        let expected = siw(&problem, max_k);

        prop_assert_eq!(&r.plan, &expected.plan);
        prop_assert_eq!((r.expanded, r.generated), (expected.expanded, expected.generated));
    }

    /// HIW without high-level features is IW at the low level: same plan,
    /// same node counts.
    #[test]
    fn hiw_without_high_level_features_is_iw(task in task(), k in 1usize..=2) {
        let problem = SasProblem::new(&task);
        let r = hiw(&problem, |_: &_, _: &mut Vec<u32>| {}, 0, 1, k);
        let expected = iw(&problem, k);

        prop_assert_eq!(&r.plan, &expected.plan);
        prop_assert_eq!((r.expanded, r.generated), (expected.expanded, expected.generated));
    }

    /// HIW with `#g` as the high-level feature returns only valid plans and
    /// none for an unsolvable task.
    #[test]
    fn hiw_plans_are_valid(task in task(), w_high in 1usize..=2, w_low in 1usize..=2) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let problem = SasProblem::new(&task);
        let goals = task.goal.len();
        let goal_count = |s: &_, out: &mut Vec<u32>| {
            out.push(u32::try_from(problem.unsatisfied_goals(s)).unwrap());
        };
        let plan = hiw(&problem, goal_count, goals + 1, w_high, w_low).plan;

        if let Some(plan) = plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }

    /// IHIW returns only valid plans and none for an unsolvable task, and
    /// the same seed gives the same result.
    #[test]
    fn ihiw_plans_are_valid_and_seeded(task in task(), seed in any::<u64>()) {
        let solvable = task.perfect_heuristic()(&task.initial).is_some();
        let problem = SasProblem::new(&task);
        let r = ihiw(&problem, 1, 1, seed);
        let again = ihiw(&problem, 1, 1, seed);

        prop_assert_eq!(&r.plan, &again.plan);
        prop_assert_eq!((r.expanded, r.generated), (again.expanded, again.generated));
        if let Some(plan) = r.plan {
            prop_assert!(solvable);
            prop_assert!(task.validate_plan(&plan).is_some());
        }
    }
}
