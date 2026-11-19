//! A native (non-PDDL) problem: fetch the key, then walk through the door.
//! Position-only novelty (IW(1)) is not enough, because after picking up the
//! key the agent must revisit cells; the pair (cell, has_key) is novel, so
//! IW(2) succeeds. Run: `cargo run --example key_door`.

use width_search::{WidthProblem, iterated_width};

const MAP: &[&str] = &[
    "#########",
    "#S..#..G#",
    "#...D...#",
    "#...#####",
    "#..K#",
    "#####",
];

#[derive(Clone, Copy, Debug)]
enum Move {
    Up,
    Down,
    Left,
    Right,
}

struct Grid {
    cells: Vec<Vec<u8>>,
    width: usize,
}

#[derive(Clone)]
struct S {
    x: usize,
    y: usize,
    key: bool,
}

impl Grid {
    fn find(&self, c: u8) -> (usize, usize) {
        for (y, row) in self.cells.iter().enumerate() {
            if let Some(x) = row.iter().position(|&b| b == c) {
                return (x, y);
            }
        }
        panic!("{} not on map", c as char)
    }
    fn at(&self, x: usize, y: usize) -> u8 {
        *self.cells.get(y).and_then(|r| r.get(x)).unwrap_or(&b'#')
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
            let (x, y) = ((s.x as isize + dx) as usize, (s.y as isize + dy) as usize);
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
    fn features(&self, s: &S, out: &mut Vec<u32>) {
        out.push((s.y * self.width + s.x) as u32);
        out.push((self.cells.len() * self.width) as u32 + u32::from(s.key));
    }
    fn num_features(&self) -> usize {
        self.cells.len() * self.width + 2
    }
    fn is_goal(&self, s: &S) -> bool {
        self.at(s.x, s.y) == b'G'
    }
}

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
