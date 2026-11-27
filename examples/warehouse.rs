//! A warehouse robot delivers parcels from the dock to their shelves.
//!
//! The robot carries one parcel at a time, so every parcel needs its own trip,
//! and the goal is a conjunction: all parcels on their shelves at once. Each
//! single delivery has low width, but the conjunction does not, so plain
//! iterated width runs out of width. Serialized IW (SIW) and BFWS use the
//! number of parcels not yet delivered (`#g`) to split the task into one
//! delivery at a time. Run: `cargo run --example warehouse`.

use width_search::{GoalCount, SearchResult, WidthProblem, bfws, iterated_width, siw};

/// The floor: `R` robot start, `a`-`c` parcels on the dock, `A`-`C` their
/// shelves, `#` racks and walls. Rows may differ in length; missing cells
/// are walls.
const FLOOR: &[&str] = &[
    "###########",
    "#R....#..A#",
    "#.###.#.###",
    "#a..#...#B#",
    "#b..###.#.#",
    "#c........#",
    "#####.###C#",
    "    #######",
];

/// What the robot can do in one step.
#[derive(Clone, Copy, Debug)]
enum Action {
    /// Move one cell up, down, left or right, given as `(dx, dy)`.
    Move(isize, isize),
    /// Pick up the parcel with this index from the robot's cell.
    Pick(usize),
    /// Put down the parcel the robot carries.
    Drop,
}

/// Where a parcel is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Place {
    /// On the floor, at this cell index.
    Cell(usize),
    /// In the robot's gripper.
    Carried,
}

/// The robot's position and every parcel's place.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct State {
    /// The robot's cell index.
    robot: usize,
    /// Each parcel's place, indexed like [`Warehouse::shelves`].
    parcels: Vec<Place>,
}

/// The parsed floor as a [`WidthProblem`].
struct Warehouse {
    /// Floor rows as bytes, indexed `[y][x]`.
    cells: Vec<Vec<u8>>,
    /// Length of the longest row; cell `(x, y)` has index `y * width + x`.
    width: usize,
    /// The start state.
    start: State,
    /// The target shelf cell of each parcel.
    shelves: Vec<usize>,
}

impl Warehouse {
    /// Parses [`FLOOR`]: parcels are lower-case letters, their shelves the
    /// matching upper-case letters.
    ///
    /// # Panics
    ///
    /// Panics if the robot or a parcel's shelf is missing.
    fn parse(floor: &[&str]) -> Self {
        let cells: Vec<Vec<u8>> = floor.iter().map(|r| r.bytes().collect()).collect();
        let width = cells.iter().map(Vec::len).max().unwrap_or(0);
        let find = |c: u8| {
            cells
                .iter()
                .enumerate()
                .find_map(|(y, row)| row.iter().position(|&b| b == c).map(|x| y * width + x))
        };

        let robot = find(b'R').expect("the floor has a robot");
        let mut parcels = Vec::new();
        let mut shelves = Vec::new();
        for letter in b'a'..=b'z' {
            let Some(cell) = find(letter) else { break };
            parcels.push(Place::Cell(cell));
            shelves.push(find(letter.to_ascii_uppercase()).expect("every parcel has a shelf"));
        }

        Self {
            cells,
            width,
            start: State { robot, parcels },
            shelves,
        }
    }

    /// Whether cell `index` can be driven on.
    fn is_free(&self, index: usize) -> bool {
        let (x, y) = (index % self.width, index / self.width);
        self.cells
            .get(y)
            .and_then(|r| r.get(x))
            .is_some_and(|&c| c != b'#' && c != b' ')
    }

    /// Number of cells, free or not.
    fn num_cells(&self) -> usize {
        self.cells.len() * self.width
    }

    /// Feature id from an index; the floor is small, so every id fits.
    fn id(index: usize) -> u32 {
        u32::try_from(index).expect("fewer than 2^32 features")
    }
}

impl WidthProblem for Warehouse {
    type State = State;
    type Action = Action;

    fn initial_state(&self) -> State {
        self.start.clone()
    }

    fn successors(&self, s: &State, out: &mut Vec<(Action, State)>) {
        for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let (x, y) = (s.robot % self.width, s.robot / self.width);
            let (Some(x), Some(y)) = (x.checked_add_signed(dx), y.checked_add_signed(dy)) else {
                continue;
            };
            let target = y * self.width + x;
            if x < self.width && self.is_free(target) {
                out.push((
                    Action::Move(dx, dy),
                    State {
                        robot: target,
                        ..s.clone()
                    },
                ));
            }
        }

        let carrying = s.parcels.iter().position(|&p| p == Place::Carried);
        match carrying {
            Some(i) => {
                let mut next = s.clone();
                next.parcels[i] = Place::Cell(s.robot);
                out.push((Action::Drop, next));
            }
            None => {
                for (i, &p) in s.parcels.iter().enumerate() {
                    if p == Place::Cell(s.robot) {
                        let mut next = s.clone();
                        next.parcels[i] = Place::Carried;
                        out.push((Action::Pick(i), next));
                    }
                }
            }
        }
    }

    // The robot's cell; and per parcel, its cell or "carried".
    fn features(&self, s: &State, out: &mut Vec<u32>) {
        let cells = self.num_cells();
        out.push(Self::id(s.robot));
        for (i, p) in s.parcels.iter().enumerate() {
            let offset = cells + i * (cells + 1);
            let place = match p {
                Place::Cell(c) => *c,
                Place::Carried => cells,
            };
            out.push(Self::id(offset + place));
        }
    }

    fn num_features(&self) -> usize {
        self.num_cells() * (1 + self.shelves.len()) + self.shelves.len()
    }

    fn is_goal(&self, s: &State) -> bool {
        self.unsatisfied_goals(s) == 0
    }
}

impl GoalCount for Warehouse {
    fn unsatisfied_goals(&self, s: &State) -> usize {
        s.parcels
            .iter()
            .zip(&self.shelves)
            .filter(|&(p, &shelf)| *p != Place::Cell(shelf))
            .count()
    }
}

/// Prints one row of the comparison table.
fn report(name: &str, r: &SearchResult<Action>) {
    let plan = r
        .plan
        .as_ref()
        .map_or("no plan".to_string(), |p| format!("{} steps", p.len()));
    println!("{name:<22} {plan:>10} {:>10} expanded", r.expanded);
}

/// The plan as one character per action: arrows for moves, the parcel's
/// letter for a pick-up, `.` for a drop.
fn render(plan: &[Action]) -> String {
    plan.iter()
        .map(|a| match *a {
            Action::Move(0, -1) => '^',
            Action::Move(0, _) => 'v',
            Action::Move(-1, _) => '<',
            Action::Move(..) => '>',
            Action::Pick(i) => char::from(b'a' + u8::try_from(i).expect("few parcels")),
            Action::Drop => '.',
        })
        .collect()
}

/// Runs iterated width, SIW and BFWS on the floor and compares them.
fn main() {
    let warehouse = Warehouse::parse(FLOOR);
    println!("{} parcels to shelve\n", warehouse.shelves.len());

    let (_, r) = iterated_width(&warehouse, 2);
    report("IW(1), IW(2)", &r);
    report("SIW, max width 2", &siw(&warehouse, 2));
    let r = bfws(&warehouse, 2);
    report("BFWS(<w_#g, #g>), k=2", &r);
    println!(
        "\nBFWS plan: {}",
        render(&r.plan.expect("BFWS is complete"))
    );
}
