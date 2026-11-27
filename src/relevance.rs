/// The relaxed-plan features `#r` counts, from one recomputation.
pub(crate) struct RelevantSet {
    /// Relevant feature ids, sorted ascending, none true where it was computed.
    features: Vec<u32>,
}

/// The `#r` bookkeeping of one search node.
#[derive(Clone)]
pub(crate) struct Relevance {
    /// Index of the [`RelevantSet`] in force for this node.
    set: usize,
    /// Bitset over the positions of that set: which features were reached.
    reached: Vec<u64>,
    /// Number of set bits in `reached`: the node's `#r`.
    pub count: usize,
}

/// State of the `#r` computation: the relaxed-plan oracle and every set it
/// produced. `None` oracle disables `#r`, which is then always 0.
pub(crate) struct RelevanceTracker<F> {
    /// Computes relaxed-plan features of a state; `None` disables `#r`.
    oracle: Option<F>,
    /// All relevant sets computed so far, indexed by [`Relevance::set`].
    sets: Vec<RelevantSet>,
    /// Reused buffer for the oracle's output.
    buf: Vec<u32>,
}

impl<F> RelevanceTracker<F> {
    /// A tracker that asks `oracle` for relaxed-plan features; `None`
    /// disables `#r`.
    pub fn new(oracle: Option<F>) -> Self {
        Self {
            oracle,
            sets: Vec::new(),
            buf: Vec::new(),
        }
    }

    /// A fresh relevant set computed at a state with features `feats`;
    /// `None` when `#r` is disabled.
    pub fn recompute<S>(&mut self, state: &S, feats: &[u32]) -> Option<Relevance>
    where
        F: FnMut(&S, &mut Vec<u32>),
    {
        let oracle = self.oracle.as_mut()?;
        self.buf.clear();
        oracle(state, &mut self.buf);
        let mut features: Vec<u32> = self
            .buf
            .iter()
            .copied()
            .filter(|f| !feats.contains(f))
            .collect();
        features.sort_unstable();
        features.dedup();

        let words = features.len().div_ceil(64);
        self.sets.push(RelevantSet { features });

        Some(Relevance {
            set: self.sets.len() - 1,
            reached: vec![0; words],
            count: 0,
        })
    }

    /// Number of features in the relevant set of `rel`: the largest `#r` a
    /// node sharing that set can reach.
    pub fn set_len(&self, rel: &Relevance) -> usize {
        self.sets[rel.set].features.len()
    }

    /// The child's relevance: the parent's, plus the relevant features true
    /// in the child (`feats`).
    pub fn extend(&self, parent: &Relevance, feats: &[u32]) -> Relevance {
        let set = &self.sets[parent.set].features;
        let mut child = parent.clone();
        for f in feats {
            let Ok(pos) = set.binary_search(f) else {
                continue;
            };
            let (word, bit) = (pos / 64, 1u64 << (pos % 64));
            if child.reached[word] & bit == 0 {
                child.reached[word] |= bit;
                child.count += 1;
            }
        }

        child
    }
}
