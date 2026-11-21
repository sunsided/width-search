//! `SasProblem::h_add` values pinned on the gripper fixtures, plus the
//! unreachable-goal and derived-default cases.

use sas_plus::fdr::SasFile;
use sas_plus::random::{RandomTaskParams, random_task};
use sas_plus::{Axiom, Effect, Fact, Operator, State, Task, Variable};
use width_search::{bfws, sas::SasProblem};

/// Parses `tests/fixtures/<path>/output.sas` into a task.
fn load(path: &str) -> Task {
    let path = format!(
        "{}/tests/fixtures/{path}/output.sas",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: SasFile = std::fs::read_to_string(path).unwrap().parse().unwrap();

    file.to_task()
}

/// `h_add` of the initial state, pinned per fixture, and 0 at the end of a
/// valid plan.
#[test]
fn h_add_is_pinned_on_the_gripper_fixtures() {
    for (path, expected) in [
        ("gripper-unit", 6),
        ("translated/gripper-2", 3),
        ("translated/gripper-4", 9),
    ] {
        let task = load(path);
        let problem = SasProblem::new(&task);
        assert_eq!(
            problem.h_add(&task.initial_state()),
            Some(expected),
            "{path}"
        );

        let plan = bfws(&problem, 2).plan.expect("gripper is solvable");
        let goal = plan
            .iter()
            .fold(task.initial_state(), |state, &op| task.apply(op, &state));
        assert_eq!(problem.h_add(&goal), Some(0), "{path}");
    }
}

/// A goal fact no rule can reach makes `h_add` `None`.
#[test]
fn h_add_is_none_when_the_relaxed_goal_is_unreachable() {
    let task = Task {
        variables: vec![Variable::new("a", 2), Variable::new("b", 2)],
        operators: vec![Operator {
            name: "set-a".into(),
            pre: vec![],
            eff: vec![Effect::new(Fact::new(0, 1))],
            cost: 1,
        }],
        initial: State(vec![0, 0]),
        goal: vec![Fact::new(0, 1), Fact::new(1, 1)],
        axioms: vec![],
    };
    let problem = SasProblem::new(&task);

    assert_eq!(problem.h_add(&task.initial_state()), None);
    assert_eq!(problem.h_add(&State(vec![0, 1])), Some(1));
}

/// No rule makes a derived variable's default true, so a goal on a default
/// that does not hold in the state is `None`, even though one operator
/// reaches it.
#[test]
fn h_add_does_not_reach_derived_defaults() {
    let mut derived = Variable::new("d", 2);
    derived.axiom_layer = Some(0);
    let task = Task {
        variables: vec![Variable::new("v", 2), derived],
        operators: vec![Operator {
            name: "set-v".into(),
            pre: vec![],
            eff: vec![Effect::new(Fact::new(0, 1))],
            cost: 1,
        }],
        initial: State(vec![0, 0]),
        goal: vec![Fact::new(1, 0)],
        axioms: vec![Axiom {
            conditions: vec![Fact::new(0, 0)],
            head: Fact::new(1, 1),
        }],
    };
    let problem = SasProblem::new(&task);

    assert_eq!(task.validate_plan(&[0]), Some(1));
    assert_eq!(problem.h_add(&task.initial_state()), None);
}

/// `h_add` sums condition costs, so a rule needing two facts of cost `c`
/// costs at least `2c`. Seventy layers of that overflow `u64`; the value
/// saturates instead.
#[test]
fn h_add_saturates_instead_of_overflowing() {
    const LAYERS: usize = 70;
    // Two variables per layer; each is set by one operator that needs both
    // variables of the layer below, so the cost doubles per layer.
    let variables = (0..2 * LAYERS)
        .map(|i| Variable::new(format!("v{i}"), 2))
        .collect();
    let operators = (0..2 * LAYERS)
        .map(|i| {
            let layer = i / 2;
            let pre = if layer == 0 {
                vec![]
            } else {
                vec![Fact::new(2 * layer - 2, 1), Fact::new(2 * layer - 1, 1)]
            };
            Operator {
                name: format!("set-{i}"),
                pre,
                eff: vec![Effect::new(Fact::new(i, 1))],
                cost: 1,
            }
        })
        .collect();
    let task = Task {
        variables,
        operators,
        initial: State(vec![0; 2 * LAYERS]),
        goal: vec![Fact::new(2 * LAYERS - 1, 1)],
        axioms: vec![],
    };
    let problem = SasProblem::new(&task);

    assert_eq!(problem.h_add(&task.initial_state()), Some(u64::MAX));
}

/// A fact that is both an operator precondition and a condition of one of its
/// effects counts once in that effect's rule. Random task seed 31 with effect
/// conditions repeats a precondition; the standard `h_add` of its reachable
/// state `[1, 0, 2, 2]` is 13, and counting the repeated fact twice gave 15.
#[test]
fn h_add_counts_a_repeated_condition_once() {
    let params = RandomTaskParams {
        max_conditions: 2,
        ..RandomTaskParams::default()
    };
    let task = random_task(31, params);
    let repeats = task.operators.iter().any(|op| {
        op.eff
            .iter()
            .any(|e| e.conditions.iter().any(|c| op.pre.contains(c)))
    });
    assert!(repeats, "seed 31 no longer repeats a precondition");

    let problem = SasProblem::new(&task);
    assert_eq!(problem.h_add(&State(vec![1, 0, 2, 2])), Some(13));
}
