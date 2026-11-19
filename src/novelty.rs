use std::collections::HashSet;

/// Records every feature tuple of size `<= k` seen so far.
///
/// The novelty of a state is the size of the smallest tuple of its features
/// that has not been seen before. Size-1 and size-2 tuples use dense bitsets
/// (`n` and `n(n-1)/2` bits); larger sizes fall back to hashing.
#[derive(Clone, Debug)]
pub struct NoveltyTable {
    /// Width bound: the largest tuple size recorded. At least 1.
    k: usize,
    /// Number of features; every feature id is below it.
    n: usize,
    /// Seen single features, indexed by feature id.
    seen1: Vec<bool>,
    /// Seen pairs as a bitset over [`NoveltyTable::pair_index`]; empty when `k < 2`.
    seen2: Vec<u64>,
    /// Seen tuples of size 3 to `k`, each sorted ascending.
    seen_hi: HashSet<Vec<u32>>,
    /// Reused buffer for the sorted, deduplicated features of the current state.
    scratch: Vec<u32>,
}

impl NoveltyTable {
    /// Creates an empty table for features `0..num_features` and tuples up to size `k`.
    ///
    /// Memory is `num_features` bools plus, for `k >= 2`, a bitset of
    /// `num_features * (num_features - 1) / 2` pairs. For `k >= 3`, every distinct
    /// tuple of size 3 to `k` seen so far is also kept in a hash set, which grows
    /// with the explored states rather than with `num_features`.
    ///
    /// # Panics
    ///
    /// Panics if `k` is 0.
    #[must_use]
    pub fn new(num_features: usize, k: usize) -> Self {
        assert!(k >= 1, "width bound must be at least 1");
        let pair_bits = if k >= 2 {
            num_features * num_features.saturating_sub(1) / 2
        } else {
            0
        };
        Self {
            k,
            n: num_features,
            seen1: vec![false; num_features],
            seen2: vec![0; pair_bits.div_ceil(64)],
            seen_hi: HashSet::new(),
            scratch: Vec::new(),
        }
    }

    /// The width bound `k` this table was created with.
    #[must_use]
    pub fn width_bound(&self) -> usize {
        self.k
    }

    /// Returns the novelty of a state with the given features (`None` if it
    /// exceeds `k`), and records all of its tuples of size `<= k`.
    ///
    /// Duplicate features are ignored.
    ///
    /// # Panics
    ///
    /// Panics if a feature is not below the `num_features` given to
    /// [`NoveltyTable::new`].
    pub fn evaluate_and_insert(&mut self, features: &[u32]) -> Option<usize> {
        let mut feats = std::mem::take(&mut self.scratch);
        feats.clear();
        feats.extend_from_slice(features);
        feats.sort_unstable();
        feats.dedup();

        let mut novelty = None;
        for &f in &feats {
            let seen = &mut self.seen1[f as usize];
            if !*seen {
                *seen = true;
                novelty.get_or_insert(1);
            }
        }

        if self.k >= 2 {
            for (i, &a) in feats.iter().enumerate() {
                for &b in &feats[i + 1..] {
                    let idx = self.pair_index(a as usize, b as usize);
                    let (word, bit) = (idx / 64, 1u64 << (idx % 64));
                    if self.seen2[word] & bit == 0 {
                        self.seen2[word] |= bit;
                        novelty.get_or_insert(2);
                    }
                }
            }
        }

        for size in 3..=self.k.min(feats.len()) {
            let mut tuple = Vec::with_capacity(size);
            if insert_combinations(&feats, size, 0, &mut tuple, &mut self.seen_hi) {
                novelty.get_or_insert(size);
            }
        }

        self.scratch = feats;
        novelty
    }

    /// Index of pair `(a, b)`, `a < b`, in the upper triangle.
    fn pair_index(&self, a: usize, b: usize) -> usize {
        a * (2 * self.n - a - 1) / 2 + (b - a - 1)
    }
}

/// Inserts every `size`-combination of `feats[start..]` (extending `tuple`);
/// returns whether any was new.
fn insert_combinations(
    feats: &[u32],
    size: usize,
    start: usize,
    tuple: &mut Vec<u32>,
    seen: &mut HashSet<Vec<u32>>,
) -> bool {
    if tuple.len() == size {
        return seen.insert(tuple.clone());
    }

    let mut any_new = false;
    let needed = size - tuple.len();
    for i in start..=feats.len() - needed {
        tuple.push(feats[i]);
        any_new |= insert_combinations(feats, size, i + 1, tuple, seen);
        tuple.pop();
    }

    any_new
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novelty_levels() {
        let mut t = NoveltyTable::new(4, 3);
        assert_eq!(t.evaluate_and_insert(&[0, 1]), Some(1));
        assert_eq!(t.evaluate_and_insert(&[2]), Some(1));
        assert_eq!(t.evaluate_and_insert(&[0, 2]), Some(2)); // new pair
        assert_eq!(t.evaluate_and_insert(&[1, 2]), Some(2));
        assert_eq!(t.evaluate_and_insert(&[0, 1, 2]), Some(3)); // only the triple is new
        assert_eq!(t.evaluate_and_insert(&[0, 1, 2]), None);
    }

    #[test]
    fn pair_index_is_a_bijection() {
        let t = NoveltyTable::new(7, 2);
        let mut idx: Vec<_> = (0..7)
            .flat_map(|a| (a + 1..7).map(move |b| (a, b)))
            .map(|(a, b)| t.pair_index(a, b))
            .collect();
        idx.sort_unstable();
        assert_eq!(idx, (0..21).collect::<Vec<_>>());
    }
}
