//! Adapter: run width-based search directly on a [`sas_plus::Task`].
//! Features are the task's facts `var = value`; plans are operator ids.

use std::sync::OnceLock;

use sas_plus::{Fact, OpId, State, Task};

use crate::{GoalCount, RelaxedPlan, WidthProblem};

/// A [`Task`] viewed as a [`WidthProblem`]: one feature per fact `var = value`,
/// and the number of unsatisfied goal facts as `#g`.
pub struct SasProblem<'a> {
    /// The wrapped task.
    task: &'a Task,
    /// Feature id of `var = 0` for each variable; `var = v` is `offsets[var] + v`.
    offsets: Vec<u32>,
    /// Total number of facts, the sum of all domain sizes.
    num_facts: usize,
    /// The task's effects and axioms as delete-free rules, for relaxed plans.
    /// Built on first use, so searches without relaxed plans never pay for it.
    rules: OnceLock<Vec<Rule>>,
}

/// A delete-free rule `conditions → head`: one effect of an operator (its
/// precondition plus the effect's conditions) or one axiom.
struct Rule {
    /// Feature ids that must all hold.
    conditions: Vec<u32>,
    /// The feature the rule makes true.
    head: u32,
    /// The operator the effect belongs to; `None` for an axiom.
    op: Option<OpId>,
    /// Cost of applying the rule: the operator cost, 0 for an axiom.
    cost: u64,
}

impl<'a> SasProblem<'a> {
    /// Wraps `task`, numbering its facts variable by variable.
    #[must_use]
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
            rules: OnceLock::new(),
        }
    }

    /// The delete-free rules, built on the first call.
    fn rules(&self) -> &[Rule] {
        self.rules.get_or_init(|| self.build_rules())
    }

    /// One rule per operator effect (precondition plus effect conditions)
    /// and one per axiom.
    fn build_rules(&self) -> Vec<Rule> {
        let fact = |f: &Fact| self.feature(f);
        let effects = self
            .task
            .operators
            .iter()
            .enumerate()
            .flat_map(|(id, op)| op.eff.iter().map(move |e| (id, op, e)));
        let mut rules: Vec<Rule> = effects
            .map(|(id, op, e)| Rule {
                conditions: op.pre.iter().chain(&e.conditions).map(fact).collect(),
                head: fact(&e.fact),
                op: Some(id),
                cost: u64::from(op.cost),
            })
            .collect();
        rules.extend(self.task.axioms.iter().map(|a| Rule {
            conditions: a.conditions.iter().map(fact).collect(),
            head: fact(&a.head),
            op: None,
            cost: 0,
        }));

        rules
    }

    /// Feature id of `fact`.
    fn feature(&self, fact: &Fact) -> u32 {
        self.offsets[fact.var] + fact.value
    }

    /// `h_add` cost of every fact from `state` and, for each fact reached by a
    /// rule, the index of its cheapest supporting rule.
    ///
    /// Unreachable facts cost `None`. Rules are relaxed until nothing
    /// improves; shortcut: `O(facts · rules)` sweeps worst case, fine for the
    /// task sizes this adapter targets, upgrade to a priority queue for large tasks.
    fn h_add(&self, state: &State) -> (Vec<Option<u64>>, Vec<Option<usize>>) {
        let mut cost = vec![None; self.num_facts];
        let mut supporter = vec![None; self.num_facts];
        for f in self.features_of(state) {
            cost[f as usize] = Some(0);
        }

        let mut changed = true;
        while changed {
            changed = false;
            for (i, rule) in self.rules().iter().enumerate() {
                let Some(pre) = rule
                    .conditions
                    .iter()
                    .try_fold(0u64, |acc, &c| cost[c as usize].map(|x| acc + x))
                else {
                    continue;
                };
                let total = pre + rule.cost;
                let head = rule.head as usize;
                if cost[head].is_none_or(|old| total < old) {
                    cost[head] = Some(total);
                    supporter[head] = Some(i);
                    changed = true;
                }
            }
        }

        (cost, supporter)
    }

    /// The feature ids true in `state`.
    fn features_of<'s>(&'s self, state: &'s State) -> impl Iterator<Item = u32> + 's {
        state.0.iter().zip(&self.offsets).map(|(&v, &o)| o + v)
    }

    /// The wrapped task, for validating or printing plans.
    #[must_use]
    pub fn task(&self) -> &Task {
        self.task
    }
}

impl WidthProblem for SasProblem<'_> {
    type State = State;
    type Action = OpId;

    fn initial_state(&self) -> State {
        self.task.initial_state()
    }
    fn successors(&self, state: &State, out: &mut Vec<(OpId, State)>) {
        out.extend(
            self.task
                .applicable_operators(state)
                .map(|op| (op, self.task.apply(op, state))),
        );
    }
    fn features(&self, state: &State, out: &mut Vec<u32>) {
        out.extend(self.features_of(state));
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

/// The relaxed plan is extracted from `h_add` best supporters, and its
/// features are all effects of its operators, as in LAPKT. Axioms on the way
/// are followed but contribute no features of their own.
impl RelaxedPlan for SasProblem<'_> {
    fn relaxed_plan_features(&self, state: &State, out: &mut Vec<u32>) {
        let (cost, supporter) = self.h_add(state);
        let goals: Vec<u32> = self.task.goal.iter().map(|f| self.feature(f)).collect();
        if goals.iter().any(|&g| cost[g as usize].is_none()) {
            return;
        }

        let mut ops = vec![false; self.task.operators.len()];
        let mut done = vec![false; self.num_facts];
        let mut stack = goals;
        while let Some(f) = stack.pop() {
            if std::mem::replace(&mut done[f as usize], true) {
                continue;
            }

            // Facts true in `state` have no supporter and end the chain.
            let Some(r) = supporter[f as usize] else {
                continue;
            };
            let rule = &self.rules()[r];
            if let Some(op) = rule.op {
                ops[op] = true;
            }

            stack.extend(&rule.conditions);
        }

        for (op, _) in ops.iter().enumerate().filter(|&(_, &used)| used) {
            out.extend(
                self.task.operators[op]
                    .eff
                    .iter()
                    .map(|e| self.feature(&e.fact)),
            );
        }
    }
}
