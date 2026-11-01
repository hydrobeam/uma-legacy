//! xorshift64 (Marsaglia's 13, 7, 17), the one pseudo-random stream the solver and its tests draw from. Its stream is
//! pinned: the schedules' iterated local search reads it (`loop_planner::schedules`), so the val/ outputs depend on it.

/// xorshift64's state; never 0 (a zero state stays zero).
pub(crate) struct XorShift64(pub(crate) u64);

impl XorShift64 {
    pub(crate) const fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A uniform in [0, 1): the top 53 bits of the next draw.
    pub(crate) fn unif(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}
