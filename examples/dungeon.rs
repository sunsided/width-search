//! Validating a generated dungeon level: can the exit be reached?
//!
//! Keys `a`-`c` open the doors `A`-`C`, and each key lies behind the previous
//! door, so the player has to walk back and forth through cells already seen.
//! Novelty over positions alone (IW(1)) prunes those revisits and declares
//! the level unsolvable. IW with larger width finds the path, but tracks
//! every pair or triple of facts. Hierarchical IW (HIW) instead treats "which
//! keys are held" as high-level state and runs IW(1) over positions inside
//! each one. IHIW finds the key flags as high-level features on its own.
//! Run: `cargo run --example dungeon`.

use width_search::{SearchResult, WidthProblem, hiw, ihiw, iterated_width};

/// The level: `@` start, `>` exit, `a`-`c` keys, `A`-`C` their doors, `#`
/// walls. Rows may differ in length; missing cells are walls.
const LEVEL: &[&str] = &[
    "###############",
    "#@....#...C..>#",
    "#.###.#.#######",
    "#.#a#.A.#c..#",
    "#.#.#.###.#.#",
    "#...#...B...#",
    "#####.###.###",
    "    #..b#",
    "    #####",
];

/// Number of key colors in [`LEVEL`].
const KEYS: usize = 3;

/// The player's position and the keys held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct State {
    /// Cell index, `y * width + x`.
    cell: usize,
    /// Bit `i` is set once key `i` (`a` + i) is picked up.
    keys: u8,
}

/// The parsed level as a [`WidthProblem`].
struct Dungeon {
    /// Level rows as bytes, indexed `[y][x]`.
    cells: Vec<Vec<u8>>,
    /// Length of the longest row.
    width: usize,
}

impl Dungeon {
    /// The byte at cell `index`; outside the level is a wall.
    fn at(&self, index: usize) -> u8 {
        let (x, y) = (index % self.width, index / self.width);
        *self.cells.get(y).and_then(|r| r.get(x)).unwrap_or(&b'#')
    }

    /// Number of cells, walls included.
    fn num_cells(&self) -> usize {
        self.cells.len() * self.width
    }

    /// Feature id from an index; the level is small, so every id fits.
    fn id(index: usize) -> u32 {
        u32::try_from(index).expect("fewer than 2^32 features")
    }

    /// The high-level features of `s`: one per key held, numbered from 0.
    fn keys_held(s: &State, out: &mut Vec<u32>) {
        out.extend((0..KEYS).filter(|&k| s.keys & (1 << k) != 0).map(Self::id));
    }
}

impl WidthProblem for Dungeon {
    type State = State;
    type Action = char;

    fn initial_state(&self) -> State {
        let cell = (0..self.num_cells())
            .find(|&i| self.at(i) == b'@')
            .expect("a start");
        State { cell, keys: 0 }
    }

    fn successors(&self, s: &State, out: &mut Vec<(char, State)>) {
        let (x, y) = (s.cell % self.width, s.cell / self.width);
        for (name, dx, dy) in [('^', 0, -1), ('v', 0, 1), ('<', -1, 0), ('>', 1, 0)] {
            let (Some(x), Some(y)) = (x.checked_add_signed(dx), y.checked_add_signed(dy)) else {
                continue;
            };
            if x >= self.width {
                continue;
            }

            let cell = y * self.width + x;
            let keys = match self.at(cell) {
                b'#' | b' ' => continue,
                door @ b'A'..=b'C' if s.keys & (1 << (door - b'A')) == 0 => continue,
                key @ b'a'..=b'c' => s.keys | (1 << (key - b'a')),
                _ => s.keys,
            };
            out.push((name, State { cell, keys }));
        }
    }

    // The player's cell, plus one feature per key held.
    fn features(&self, s: &State, out: &mut Vec<u32>) {
        out.push(Self::id(s.cell));
        Self::keys_held(s, out);
        for f in out.iter_mut().skip(1) {
            *f += Self::id(self.num_cells());
        }
    }

    fn num_features(&self) -> usize {
        self.num_cells() + KEYS
    }

    fn is_goal(&self, s: &State) -> bool {
        self.at(s.cell) == b'>'
    }
}

/// Prints one row of the comparison table.
fn report(name: &str, r: &SearchResult<char>) {
    let plan = r
        .plan
        .as_ref()
        .map_or("unsolvable?".to_string(), |p| format!("{} steps", p.len()));
    println!("{name:<28} {plan:>12} {:>8} expanded", r.expanded);
}

/// Checks the level with flat and hierarchical searches.
fn main() {
    let cells: Vec<Vec<u8>> = LEVEL.iter().map(|r| r.bytes().collect()).collect();
    let width = cells.iter().map(Vec::len).max().unwrap_or(0);
    let dungeon = Dungeon { cells, width };

    let (_, r) = iterated_width(&dungeon, 1);
    report("IW(1)", &r);
    let (k, r) = iterated_width(&dungeon, 3);
    let k = k.expect("the level has width 3 at most");
    report(&format!("IW(1) .. IW({k})"), &r);
    let r = hiw(&dungeon, Dungeon::keys_held, KEYS, 1, 1);
    report("HIW(1, 1), keys as high level", &r);
    report("IHIW(1, 1), finds the keys", &ihiw(&dungeon, 1, 1, 7));

    let plan: String = r.plan.expect("the level is solvable").into_iter().collect();
    println!("\nHIW path: {plan}");
}
