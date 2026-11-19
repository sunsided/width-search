//! Node counts on the translated fixtures, pinned next to LAPKT's.
//!
//! LAPKT 0.2.1 (Tarski grounder) numbers come from `scripts/lapkt-compare.sh`;
//! the README explains how its planners map onto this crate's searches.
//! Grounding differs (Fast Downward facts here, STRIPS atoms in LAPKT), so
//! only the IW rows are expected to agree closely.

use sas_plus::Task;
use sas_plus::fdr::SasFile;
use width_search::sas::SasProblem;
use width_search::{SearchResult, bfws, bfws_r, iterated_width, iw, siw};

/// One pinned measurement: our expanded count and LAPKT's for the same task.
struct Row {
    /// Fixture name under `tests/fixtures/translated/`.
    task: &'static str,
    /// Which search was run, as named in the README table.
    search: &'static str,
    /// Nodes expanded by this crate.
    expanded: usize,
    /// Nodes expanded by the matching LAPKT planner.
    lapkt: usize,
    /// Whether a plan is expected.
    solved: bool,
}

/// The pinned measurements. Heavy runs (SIW and IW(2) on logistics-e) are
/// left out to keep debug-mode tests fast; the README lists them.
const ROWS: &[Row] = &[
    row("gripper-2", "iw1", 6, 6, false),
    row("gripper-2", "iw2", 13, 13, true),
    row("gripper-2", "siw", 13, 13, true),
    row("gripper-2", "bfws", 7, 4, true),
    row("gripper-2", "bfws_r", 7, 4, true),
    row("sussman-anomaly", "iw1", 7, 7, false),
    row("sussman-anomaly", "iw2", 16, 16, true),
    row("sussman-anomaly", "siw", 17, 12, true),
    row("sussman-anomaly", "bfws", 8, 8, true),
    row("sussman-anomaly", "bfws_r", 11, 6, true),
    row("gripper-4", "iw1", 10, 10, false),
    row("gripper-4", "iw2", 72, 72, false),
    row("gripper-4", "siw", 71, 71, true),
    row("gripper-4", "bfws", 29, 43, true),
    row("gripper-4", "bfws_r", 27, 33, true),
    row("bw-large-a", "iw1", 49, 49, false),
    row("bw-large-a", "iw2", 1913, 1914, false),
    row("bw-large-a", "siw", 74, 16, true),
    row("bw-large-a", "bfws", 70, 48, true),
    row("bw-large-a", "bfws_r", 66, 102, true),
    row("logistics-e", "iw1", 46, 46, false),
    row("logistics-e", "bfws", 14915, 17999, true),
    row("logistics-e", "bfws_r", 4416, 4399, true),
];

/// A [`Row`] from positional values, to keep the table compact.
const fn row(
    task: &'static str,
    search: &'static str,
    expanded: usize,
    lapkt: usize,
    solved: bool,
) -> Row {
    Row {
        task,
        search,
        expanded,
        lapkt,
        solved,
    }
}

/// Parses `tests/fixtures/translated/<name>/output.sas` into a task.
fn load(name: &str) -> Task {
    let path = format!(
        "{}/tests/fixtures/translated/{name}/output.sas",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: SasFile = std::fs::read_to_string(path).unwrap().parse().unwrap();
    file.to_task()
}

/// Runs `search` with the same width bound as the LAPKT planner it is
/// compared with (`--iw_bound 2`, `--max_novelty 2`).
fn run(problem: &SasProblem<'_>, search: &str) -> SearchResult<usize> {
    match search {
        "iw1" => iw(problem, 1),
        "iw2" => iterated_width(problem, 2).1,
        "siw" => siw(problem, 2),
        "bfws" => bfws(problem, 2),
        "bfws_r" => bfws_r(problem, 2),
        _ => unreachable!("unknown search {search}"),
    }
}

/// Every search keeps its pinned expanded count and returns a valid plan
/// exactly when one is expected.
#[test]
fn node_counts_match_the_pinned_table() {
    for r in ROWS {
        let task = load(r.task);
        let result = run(&SasProblem::new(&task), r.search);
        let label = format!("{} {} (LAPKT: {})", r.task, r.search, r.lapkt);

        assert_eq!(result.expanded, r.expanded, "{label}");
        assert_eq!(result.plan.is_some(), r.solved, "{label}");
        if let Some(plan) = result.plan {
            assert!(task.validate_plan(&plan).is_some(), "{label}: invalid plan");
        }
    }
}
