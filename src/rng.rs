/// A small `SplitMix64` generator: seeded, fast, and identical on every
/// platform, so a seed fixes a search's random choices.
#[derive(Clone, Debug)]
pub(crate) struct SplitMix64 {
    /// The generator state, advanced by a fixed odd constant per draw.
    state: u64,
}

impl SplitMix64 {
    /// A generator starting from `seed`.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`.
    ///
    /// # Panics
    ///
    /// Panics if `n` is 0.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "cannot draw from an empty range");
        // shortcut: modulo bias is negligible for ranges far below 2^64.
        usize::try_from(self.next_u64() % n as u64).expect("below a usize")
    }

    /// A number in `[0, 1)` with 32 bits of resolution.
    pub fn unit(&mut self) -> f64 {
        let high = u32::try_from(self.next_u64() >> 32).expect("32 high bits fit a u32");
        f64::from(high) / 4_294_967_296.0
    }
}
