//! Property tests: [`NoveltyTable`] agrees with a naive oracle that stores
//! every feature tuple of size `<= k` in one hash set.

use std::collections::HashSet;

use proptest::prelude::*;
use width_search::NoveltyTable;

/// Largest width bound exercised; tuple counts grow as `C(features, k)`.
const MAX_K: usize = 4;

/// Largest number of features per state; keeps `C(8, 4) = 70` tuples per insert.
const MAX_STATE_FEATURES: usize = 8;

/// Reference implementation of novelty: the size of the smallest tuple of the
/// state's (deduplicated) features that has not been seen, or `None` above `k`.
struct Oracle {
    /// Width bound; tuples larger than this are never recorded.
    k: usize,
    /// Every sorted tuple of size `1..=k` recorded so far.
    seen: HashSet<Vec<u32>>,
}

impl Oracle {
    /// Creates an empty oracle for width bound `k`.
    fn new(k: usize) -> Self {
        Self {
            k,
            seen: HashSet::new(),
        }
    }

    /// Records every tuple of `features` and returns the novelty, like
    /// [`NoveltyTable::evaluate_and_insert`].
    fn evaluate_and_insert(&mut self, features: &[u32]) -> Option<usize> {
        let mut feats = features.to_vec();
        feats.sort_unstable();
        feats.dedup();

        let mut novelty = None;
        for size in 1..=self.k.min(feats.len()) {
            for tuple in combinations(&feats, size) {
                if self.seen.insert(tuple) {
                    novelty.get_or_insert(size);
                }
            }
        }

        novelty
    }
}

/// All `size`-element combinations of `items`, each in the order of `items`.
fn combinations(items: &[u32], size: usize) -> Vec<Vec<u32>> {
    if size == 0 {
        return vec![Vec::new()];
    }

    let mut out = Vec::new();
    for (i, &first) in items.iter().enumerate() {
        for mut rest in combinations(&items[i + 1..], size - 1) {
            rest.insert(0, first);
            out.push(rest);
        }
    }

    out
}

/// A feature count, a width bound and a sequence of states over those features.
fn case() -> impl Strategy<Value = (usize, usize, Vec<Vec<u32>>)> {
    (1usize..=12, 1..=MAX_K).prop_flat_map(|(n, k)| {
        let max_feature = u32::try_from(n).expect("n is at most 12");
        let state = prop::collection::vec(0..max_feature, 0..=MAX_STATE_FEATURES);

        (Just(n), Just(k), prop::collection::vec(state, 1..24))
    })
}

proptest! {
    /// Every insert returns the oracle's novelty, so the bitset layout for
    /// pairs and the hashed tuples above size 2 lose and invent nothing.
    #[test]
    fn matches_the_tuple_set_oracle((n, k, states) in case()) {
        let mut table = NoveltyTable::new(n, k);
        let mut oracle = Oracle::new(k);

        for state in &states {
            prop_assert_eq!(table.evaluate_and_insert(state), oracle.evaluate_and_insert(state));
        }
    }

    /// Re-inserting a state that was just inserted is never novel.
    #[test]
    fn repeated_state_is_not_novel((n, k, states) in case()) {
        let mut table = NoveltyTable::new(n, k);

        for state in &states {
            table.evaluate_and_insert(state);
            prop_assert_eq!(table.evaluate_and_insert(state), None);
        }
    }
}
