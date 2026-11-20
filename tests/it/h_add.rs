//! `SasProblem::h_add` values pinned on the gripper fixtures, plus the
//! unreachable-goal and derived-default cases.

use sas_plus::fdr::SasFile;
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
