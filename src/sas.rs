//! Adapter: run width-based search directly on a [`sas_plus::Task`].
//! Features are the task's facts `var = value`; plans are operator ids.

use sas_plus::{OpId, State, Task};

use crate::{GoalCount, WidthProblem};

pub struct SasProblem<'a> {
    task: &'a Task,
    offsets: Vec<u32>,
    num_facts: usize,
}

impl<'a> SasProblem<'a> {
    pub fn new(task: &'a Task) -> Self {
        let mut offsets = Vec::with_capacity(task.num_variables());
        let mut n = 0u32;
        for v in &task.variables {
            offsets.push(n);
            n += v.domain_size;
        }
        Self {
            task,
            offsets,
            num_facts: n as usize,
        }
    }

    pub fn task(&self) -> &Task {
        self.task
    }
}

impl WidthProblem for SasProblem<'_> {
    type State = State;
    type Action = OpId;

    fn initial_state(&self) -> State {
        self.task.initial.clone()
    }
    fn successors(&self, state: &State, out: &mut Vec<(OpId, State)>) {
        out.extend(
            self.task
                .applicable_operators(state)
                .map(|op| (op, self.task.apply(op, state))),
        );
    }
    fn features(&self, state: &State, out: &mut Vec<u32>) {
        out.extend(state.0.iter().zip(&self.offsets).map(|(&v, &o)| o + v));
    }
    fn num_features(&self) -> usize {
        self.num_facts
    }
    fn is_goal(&self, state: &State) -> bool {
        self.task.is_goal(state)
    }
}

impl GoalCount for SasProblem<'_> {
    fn unsatisfied_goals(&self, state: &State) -> usize {
        self.task
            .goal
            .iter()
            .filter(|f| state.0[f.var] != f.value)
            .count()
    }
}
