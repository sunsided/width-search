//! A native (non-PDDL) problem: fetch the key, then walk through the door.
//! Position-only novelty (IW(1)) is not enough, because after picking up the
//! key the agent must revisit cells; the pair (cell, `has_key`) is novel, so
//! IW(2) succeeds. Run: `cargo run --example key_door`.

use width_search::{WidthProblem, iterated_width};

/// The level: `S` start, `K` key, `D` door (passable with the key), `G` goal,
/// `#` wall. Rows may differ in length; missing cells are walls.
const MAP: &[&str] = &[
    "#########",
    "#S..#..G#",
    "#...D...#",
    "#...#####",
    "#..K#",
    "#####",
];

/// One step in a grid direction.
#[derive(Clone, Copy, Debug)]
enum Move {
    /// One row up (`y - 1`).
    Up,
    /// One row down (`y + 1`).
    Down,
    /// One column left (`x - 1`).
    Left,
    /// One column right (`x + 1`).
    Right,
}

/// The parsed level, as a [`WidthProblem`] over positions and key possession.
struct Grid {
    /// Map rows as bytes, indexed `[y][x]`.
    cells: Vec<Vec<u8>>,
    /// Length of the longest row; positions are numbered `y * width + x`.
    width: usize,
}

/// The agent's state.
#[derive(Clone)]
struct S {
    /// Column.
    x: usize,
    /// Row.
    y: usize,
    /// Whether the key has been picked up.
    key: bool,
}

impl Grid {
    /// Position `(x, y)` of the first cell holding `c`.
    ///
    /// # Panics
    ///
    /// Panics if `c` is not on the map.
    fn find(&self, c: u8) -> (usize, usize) {
        for (y, row) in self.cells.iter().enumerate() {
            if let Some(x) = row.iter().position(|&b| b == c) {
                return (x, y);
            }
        }
        panic!("{} not on map", char::from(c))
    }

    /// The cell at `(x, y)`; outside the map is a wall.
    fn at(&self, x: usize, y: usize) -> u8 {
        *self.cells.get(y).and_then(|r| r.get(x)).unwrap_or(&b'#')
    }

    /// Feature id for `index`; the map is small, so every id fits.
    fn feature(index: usize) -> u32 {
        u32::try_from(index).expect("map has fewer than 2^32 cells")
    }
}

impl WidthProblem for Grid {
    type State = S;
    type Action = Move;

    fn initial_state(&self) -> S {
        let (x, y) = self.find(b'S');
        S { x, y, key: false }
    }

    fn successors(&self, s: &S, out: &mut Vec<(Move, S)>) {
        for (m, dx, dy) in [
            (Move::Up, 0, -1),
            (Move::Down, 0, 1),
            (Move::Left, -1, 0),
            (Move::Right, 1, 0),
        ] {
            // The border is all walls, so stepping below 0 never happens on a valid map.
            let (Some(x), Some(y)) = (s.x.checked_add_signed(dx), s.y.checked_add_signed(dy))
            else {
                continue;
            };

            match self.at(x, y) {
                b'#' => {}
                b'D' if !s.key => {}
                c => out.push((
                    m,
                    S {
                        x,
                        y,
                        key: s.key || c == b'K',
                    },
                )),
            }
        }
    }

    // One feature per position, plus one each for "no key" and "key".
    fn features(&self, s: &S, out: &mut Vec<u32>) {
        let positions = self.cells.len() * self.width;

        out.push(Self::feature(s.y * self.width + s.x));
        out.push(Self::feature(positions + usize::from(s.key)));
    }

    fn num_features(&self) -> usize {
        self.cells.len() * self.width + 2
    }

    fn is_goal(&self, s: &S) -> bool {
        self.at(s.x, s.y) == b'G'
    }
}

/// Solves the level with iterated width and prints the width it needed.
fn main() {
    let cells: Vec<Vec<u8>> = MAP.iter().map(|r| r.bytes().collect()).collect();
    let width = cells.iter().map(Vec::len).max().unwrap();
    let grid = Grid { cells, width };

    let (k, r) = iterated_width(&grid, 3);
    println!(
        "solved with width {k:?}: {} expanded, {} generated",
        r.expanded, r.generated
    );
    println!("plan: {:?}", r.plan.expect("solvable"));
}
