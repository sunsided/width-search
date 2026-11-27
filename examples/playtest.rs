//! An automated playtester checks that the exit of a level can be reached.
//!
//! The level is a black box to the search: a simulator that applies moves.
//! It is a building of rooms joined by doorways, and only some doorways are
//! open. Rollout IW explores it with depth-first rollouts, which suits
//! simulators whose breadth-first layers get wide. π-IW steers the rollouts
//! with a policy, here a hand-written one that prefers moves towards the
//! exit (a trained network would fit the same slot). π-HIW also treats the
//! current room as high-level state, so it explores one room at a time.
//! Every search is randomized, so the table shows averages over seeds.
//! Run: `cargo run --example playtest`.

use width_search::{SearchResult, WidthProblem, pi_hiw, pi_iw, rollout_iw};

/// Rooms across.
const ROOMS_X: usize = 4;
/// Rooms down.
const ROOMS_Y: usize = 3;
/// Cells per room side, walls included.
const ROOM: usize = 8;
/// Level width in cells, closing wall included.
const W: usize = ROOMS_X * ROOM + 1;
/// Level height in cells, closing wall included.
const H: usize = ROOMS_Y * ROOM + 1;

/// Open doorways, as pairs of neighboring room indices `ry * ROOMS_X + rx`.
const DOORS: &[(usize, usize)] = &[
    (0, 1),
    (1, 5),
    (5, 4),
    (4, 8),
    (8, 9),
    (9, 10),
    (10, 6),
    (6, 7),
    (7, 11),
];

/// The simulator: wall layout, start and exit.
struct Level {
    /// Whether each cell `y * W + x` is a wall.
    wall: Vec<bool>,
    /// Start cell.
    start: usize,
    /// Exit cell.
    exit: usize,
}

impl Level {
    /// Builds the rooms, opens the [`DOORS`], and puts the start in the
    /// top-left room and the exit in the bottom-right one.
    fn build() -> Self {
        let mut wall = vec![false; W * H];
        for y in 0..H {
            for x in 0..W {
                wall[y * W + x] = x % ROOM == 0 || y % ROOM == 0;
            }
        }

        for &(a, b) in DOORS {
            let (ax, ay) = (a % ROOMS_X, a / ROOMS_X);
            let (bx, by) = (b % ROOMS_X, b / ROOMS_X);
            let (x, y) = if ay == by {
                (ax.max(bx) * ROOM, ay * ROOM + ROOM / 2)
            } else {
                (ax * ROOM + ROOM / 2, ay.max(by) * ROOM)
            };
            wall[y * W + x] = false;
        }

        let center =
            |r: usize| ((r / ROOMS_X) * ROOM + ROOM / 2) * W + (r % ROOMS_X) * ROOM + ROOM / 2;
        Self {
            wall,
            start: center(0),
            exit: center(ROOMS_X * ROOMS_Y - 1),
        }
    }

    /// Manhattan distance from `cell` to the exit.
    fn distance_to_exit(&self, cell: usize) -> usize {
        (cell % W).abs_diff(self.exit % W) + (cell / W).abs_diff(self.exit / W)
    }

    /// The room `cell` lies in; a doorway counts as the room to its right
    /// or below.
    fn room(cell: usize) -> u32 {
        let (rx, ry) = ((cell % W) / ROOM, (cell / W) / ROOM);
        u32::try_from(ry.min(ROOMS_Y - 1) * ROOMS_X + rx.min(ROOMS_X - 1)).expect("few rooms")
    }
}

impl WidthProblem for Level {
    type State = usize;
    type Action = char;

    fn initial_state(&self) -> usize {
        self.start
    }

    fn successors(&self, &cell: &usize, out: &mut Vec<(char, usize)>) {
        let moves = [
            ('^', cell - W),
            ('v', cell + W),
            ('<', cell - 1),
            ('>', cell + 1),
        ];
        out.extend(moves.into_iter().filter(|&(_, next)| !self.wall[next]));
    }

    // The player's cell.
    fn features(&self, &cell: &usize, out: &mut Vec<u32>) {
        out.push(u32::try_from(cell).expect("few cells"));
    }

    fn num_features(&self) -> usize {
        W * H
    }

    fn is_goal(&self, &cell: &usize) -> bool {
        cell == self.exit
    }
}

/// Average expansions and plan length of `search` over `SEEDS` seeds.
fn average(name: &str, search: impl Fn(u64) -> SearchResult<char>) {
    const SEEDS: u64 = 20;
    let (mut expanded, mut steps) = (0, 0);
    for seed in 0..SEEDS {
        let r = search(seed);
        expanded += r.expanded;
        steps += r.plan.expect("the exit is reachable").len();
    }

    let per_seed = |total: usize| total / usize::try_from(SEEDS).expect("few seeds");
    println!(
        "{name:<26} {:>6} steps {:>8} expanded",
        per_seed(steps),
        per_seed(expanded)
    );
}

/// Plays the level with each rollout-based search.
fn main() {
    let level = Level::build();
    let towards_exit = |&cell: &usize, succ: &[(char, usize)], out: &mut Vec<f64>| {
        let here = level.distance_to_exit(cell);
        out.extend(succ.iter().map(|&(_, next)| {
            if level.distance_to_exit(next) < here {
                4.0
            } else {
                1.0
            }
        }));
    };
    let room = |&cell: &usize, out: &mut Vec<u32>| out.push(Level::room(cell));
    let rooms = ROOMS_X * ROOMS_Y;

    println!("averages over 20 seeds\n");
    average("Rollout IW(1)", |seed| rollout_iw(&level, seed));
    average("π-IW, towards the exit", |seed| {
        pi_iw(&level, towards_exit, seed)
    });
    average("π-HIW, rooms as high level", |seed| {
        pi_hiw(&level, room, rooms, towards_exit, 0.005, seed)
    });
}
