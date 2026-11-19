//! Fuzzes [`NoveltyTable`] with arbitrary in-range states: inserting never
//! panics, novelty never exceeds the width bound, and re-inserting a state is
//! never novel.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use width_search::NoveltyTable;

/// Largest width bound; higher bounds blow up the tuple count per insert.
const MAX_K: usize = 4;

/// Largest number of features per state, so one insert stays under `C(10, 4)` tuples.
const MAX_STATE_FEATURES: usize = 10;

/// A table configuration and a stream of states to insert into it.
#[derive(Arbitrary, Debug)]
struct Input {
    /// Feature count minus one; features are taken modulo the count.
    features: u8,
    /// Width bound minus one, taken modulo [`MAX_K`].
    k: u8,
    /// States as raw feature ids, mapped into range before insertion.
    states: Vec<Vec<u8>>,
}

fuzz_target!(|input: Input| {
    let num_features = usize::from(input.features) + 1;
    let k = usize::from(input.k) % MAX_K + 1;
    let mut table = NoveltyTable::new(num_features, k);

    for raw in &input.states {
        let state = in_range(raw, num_features);
        if let Some(novelty) = table.evaluate_and_insert(&state) {
            assert!(
                (1..=k).contains(&novelty),
                "novelty {novelty} outside 1..={k}"
            );
        }

        assert_eq!(table.evaluate_and_insert(&state), None, "repeat was novel");
    }
});

/// Maps raw ids into `0..num_features` and caps the state at [`MAX_STATE_FEATURES`].
fn in_range(raw: &[u8], num_features: usize) -> Vec<u32> {
    raw.iter()
        .take(MAX_STATE_FEATURES)
        .map(|&f| u32::from(f) % u32::try_from(num_features).expect("at most 256 features"))
        .collect()
}
