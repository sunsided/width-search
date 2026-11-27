//! Goal-oriented action planning (GOAP) for a game NPC that must craft a
//! sword and bring it back to camp.
//!
//! The world is a small STRIPS task: facts such as "at the mine" or "has a
//! stick", and actions with preconditions, add and delete effects. Crafting
//! consumes ingredients, so the NPC has to gather sticks twice and visit the
//! forest, the mine and the forge in a sensible order. Novelty runs over the
//! true facts. BFWS with relaxed-plan counts (`#r`) and SIW+ also need to know
//! which facts lead towards the goal; this example computes a delete-relaxed
//! plan with `h_add` best supporters, the same idea as the `sas-plus`
//! adapter. Run: `cargo run --example crafting`.

use width_search::{
    GoalCount, RelaxedPlan, SearchResult, WidthProblem, bfws, bfws_r, iterated_width, siw, siw_plus,
};

/// The facts of the world, one feature id each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
enum Fact {
    /// The NPC stands at the camp.
    AtCamp,
    /// The NPC stands in the forest.
    AtForest,
    /// The NPC stands at the mine.
    AtMine,
    /// The NPC stands at the forge.
    AtForge,
    /// The NPC holds a stick.
    Stick,
    /// The NPC holds a stone.
    Stone,
    /// The NPC holds an axe.
    Axe,
    /// The NPC holds wood.
    Wood,
    /// The NPC holds a pickaxe.
    Pickaxe,
    /// The NPC holds iron ore.
    Ore,
    /// The NPC holds an iron ingot.
    Iron,
    /// The NPC holds the sword.
    Sword,
}

/// Number of [`Fact`]s.
const NUM_FACTS: usize = Fact::Sword as usize + 1;

/// A STRIPS action.
struct Action {
    /// Name shown in the plan.
    name: &'static str,
    /// Facts that must hold.
    pre: &'static [Fact],
    /// Facts made true.
    add: &'static [Fact],
    /// Facts made false.
    del: &'static [Fact],
}

/// Shorthand for an [`Action`] in [`ACTIONS`].
const fn act(
    name: &'static str,
    pre: &'static [Fact],
    add: &'static [Fact],
    del: &'static [Fact],
) -> Action {
    Action {
        name,
        pre,
        add,
        del,
    }
}

use Fact::{AtCamp, AtForest, AtForge, AtMine, Axe, Iron, Ore, Pickaxe, Stick, Stone, Sword, Wood};

/// The NPC's repertoire. Paths: camp to forest and to mine, forest to forge,
/// mine to forge.
const ACTIONS: &[Action] = &[
    act("walk camp->forest", &[AtCamp], &[AtForest], &[AtCamp]),
    act("walk forest->camp", &[AtForest], &[AtCamp], &[AtForest]),
    act("walk camp->mine", &[AtCamp], &[AtMine], &[AtCamp]),
    act("walk mine->camp", &[AtMine], &[AtCamp], &[AtMine]),
    act("walk forest->forge", &[AtForest], &[AtForge], &[AtForest]),
    act("walk forge->forest", &[AtForge], &[AtForest], &[AtForge]),
    act("walk mine->forge", &[AtMine], &[AtForge], &[AtMine]),
    act("walk forge->mine", &[AtForge], &[AtMine], &[AtForge]),
    act("pick up stick", &[AtForest], &[Stick], &[]),
    act("pick up stone", &[AtMine], &[Stone], &[]),
    act("craft axe", &[Stick, Stone], &[Axe], &[Stick, Stone]),
    act("chop wood", &[AtForest, Axe], &[Wood], &[]),
    act(
        "craft pickaxe",
        &[Stick, Stone, Wood],
        &[Pickaxe],
        &[Stick, Stone],
    ),
    act("mine ore", &[AtMine, Pickaxe], &[Ore], &[]),
    act("smelt iron", &[AtForge, Ore, Wood], &[Iron], &[Ore, Wood]),
    act(
        "forge sword",
        &[AtForge, Iron, Stick],
        &[Sword],
        &[Iron, Stick],
    ),
];

/// The goal: the sword, back at camp.
const GOAL: &[Fact] = &[Sword, AtCamp];

/// A world state: which facts hold.
type State = [bool; NUM_FACTS];

/// The crafting task as a [`WidthProblem`] over [`ACTIONS`].
struct Crafting;

impl Crafting {
    /// Whether every fact in `facts` holds in `s`.
    fn holds(s: &State, facts: &[Fact]) -> bool {
        facts.iter().all(|&f| s[f as usize])
    }
}

impl WidthProblem for Crafting {
    type State = State;
    type Action = usize;

    fn initial_state(&self) -> State {
        let mut s = [false; NUM_FACTS];
        s[AtCamp as usize] = true;
        s
    }

    fn successors(&self, s: &State, out: &mut Vec<(usize, State)>) {
        for (i, a) in ACTIONS.iter().enumerate() {
            if !Self::holds(s, a.pre) {
                continue;
            }

            let mut next = *s;
            for &f in a.del {
                next[f as usize] = false;
            }
            for &f in a.add {
                next[f as usize] = true;
            }
            out.push((i, next));
        }
    }

    // One feature per fact that holds.
    fn features(&self, s: &State, out: &mut Vec<u32>) {
        out.extend((0u32..).zip(s).filter(|&(_, holds)| *holds).map(|(f, _)| f));
    }

    fn num_features(&self) -> usize {
        NUM_FACTS
    }

    fn is_goal(&self, s: &State) -> bool {
        Self::holds(s, GOAL)
    }
}

impl GoalCount for Crafting {
    fn unsatisfied_goals(&self, s: &State) -> usize {
        GOAL.iter().filter(|&&f| !s[f as usize]).count()
    }
}

impl RelaxedPlan for Crafting {
    /// Pushes the add effects of a delete-relaxed plan for [`GOAL`]: `h_add`
    /// costs with best supporters, then the supporters needed by the goal,
    /// traced backwards.
    fn relaxed_plan_features(&self, s: &State, out: &mut Vec<u32>) {
        let (cost, supporter) = h_add(s);
        if GOAL.iter().any(|&g| cost[g as usize].is_none()) {
            return;
        }

        let mut needed: Vec<Fact> = GOAL.to_vec();
        let mut used = [false; ACTIONS.len()];
        while let Some(f) = needed.pop() {
            let Some(a) = supporter[f as usize] else {
                continue; // true in `s` already
            };
            if !std::mem::replace(&mut used[a], true) {
                needed.extend(ACTIONS[a].pre);
                out.extend(ACTIONS[a].add.iter().map(|&g| g as u32));
            }
        }
    }
}

/// `h_add` from `s`: the cost of each fact (`None` if unreachable even
/// without deletes) and the action that reaches it most cheaply.
fn h_add(s: &State) -> ([Option<u32>; NUM_FACTS], [Option<usize>; NUM_FACTS]) {
    let mut cost: [Option<u32>; NUM_FACTS] = std::array::from_fn(|f| s[f].then_some(0));
    let mut supporter = [None; NUM_FACTS];
    let mut changed = true;
    while changed {
        changed = false;
        for (i, a) in ACTIONS.iter().enumerate() {
            let Some(pre) = a.pre.iter().map(|&f| cost[f as usize]).sum::<Option<u32>>() else {
                continue;
            };
            for &f in a.add {
                if cost[f as usize].is_none_or(|c| pre + 1 < c) {
                    cost[f as usize] = Some(pre + 1);
                    supporter[f as usize] = Some(i);
                    changed = true;
                }
            }
        }
    }

    (cost, supporter)
}

/// Prints one row of the comparison table.
fn report(name: &str, r: &SearchResult<usize>) {
    let plan = r
        .plan
        .as_ref()
        .map_or("no plan".to_string(), |p| format!("{} steps", p.len()));
    println!("{name:<24} {plan:>10} {:>8} expanded", r.expanded);
}

/// Plans with every goal-directed search and prints the best plan found.
fn main() {
    let (_, r) = iterated_width(&Crafting, 2);
    report("IW(1), IW(2)", &r);
    report("SIW, max width 2", &siw(&Crafting, 2));
    report("SIW+, max width 2", &siw_plus(&Crafting, 2));
    report("BFWS(<w_#g, #g>), k=1", &bfws(&Crafting, 1));
    let r = bfws_r(&Crafting, 1);
    report("BFWS(<w_#g,#r, #g>), k=1", &r);

    println!("\nBFWS with #r plan:");
    for a in r.plan.expect("BFWS is complete") {
        println!("  {}", ACTIONS[a].name);
    }
}
