//! Benchmarks for width-based search on Fast Downward translator output, and
//! for the novelty table that dominates its cost.

use std::hint::black_box;

use criterion::Criterion;
use sas_plus::Task;
use sas_plus::fdr::SasFile;
use width_search::{NoveltyTable, bfws, iterated_width, sas::SasProblem};

/// Runs every benchmark in this file with criterion's command-line flags.
fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    bench_iterated_width(&mut criterion);
    bench_bfws(&mut criterion);
    bench_novelty_table(&mut criterion);
    criterion.final_summary();
}

/// Loads the translated fixture `name` as a task.
///
/// # Panics
///
/// Panics if the fixture is missing or does not parse; benches need a git checkout,
/// because the fixtures are not packaged.
fn load(name: &str) -> Task {
    let path = format!(
        "{}/tests/fixtures/translated/{name}/output.sas",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: SasFile = std::fs::read_to_string(path)
        .expect("fixture exists")
        .parse()
        .expect("fixture parses");

    file.to_task().expect("fixture is a valid task")
}

/// Iterated width on gripper-2, whose single-atom goal needs width 2.
fn bench_iterated_width(criterion: &mut Criterion) {
    let task = load("gripper-2");
    let problem = SasProblem::new(&task);

    criterion.bench_function("iterated_width/gripper-2", |b| {
        b.iter(|| iterated_width(black_box(&problem), 2));
    });
}

/// BFWS(2) on the larger translated benchmarks.
fn bench_bfws(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("bfws");
    for name in ["gripper-4", "logistics-e", "bw-large-a"] {
        let task = load(name);
        let problem = SasProblem::new(&task);

        group.bench_function(name, |b| b.iter(|| bfws(black_box(&problem), 2)));
    }

    group.finish();
}

/// Inserting a fixed stream of 8-feature states into a fresh table, per width bound.
fn bench_novelty_table(criterion: &mut Criterion) {
    const FEATURES: u32 = 64;
    let states: Vec<Vec<u32>> = (0..256u32)
        .map(|i| (0..8).map(|j| (i * 7 + j * 13) % FEATURES).collect())
        .collect();

    let mut group = criterion.benchmark_group("novelty_table");
    for k in 1..=3 {
        group.bench_function(format!("k={k}"), |b| {
            b.iter(|| {
                let mut table = NoveltyTable::new(FEATURES as usize, k);
                for state in &states {
                    black_box(table.evaluate_and_insert(state));
                }
            });
        });
    }

    group.finish();
}
