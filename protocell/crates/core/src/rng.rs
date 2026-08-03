//! Counter-based PRNG (Philox4x32-10) — the mechanism that makes individuation affordable.
//!
//! # The load-bearing constraint of the whole project
//!
//! Every random draw in Protocell is a **pure function of its address**:
//!
//! ```text
//!     (seed, stream, step, entity_id)  ->  [u32; 4]
//! ```
//!
//! There is no sequential RNG state anywhere. Nothing is "advanced". This is what
//! lets the event log stay small: reconstructible outcomes are *not logged*, they are
//! recomputed on demand from the address. A diffusion hop, a bind attempt that failed,
//! the exact uniform that decided a reaction channel — all recoverable without
//! replaying the trajectory, because the address alone determines the bits.
//!
//! Two rules follow, and they are hard constraints on every system built on top:
//!
//! 1. **No state-dependent RNG consumption.** The number of draws taken at
//!    `(step, entity)` must not depend on the branch taken. If entity A consumes
//!    3 uniforms on one path and 5 on another, the address space shifts under you and
//!    replay diverges. Draw a fixed budget, discard the unused.
//! 2. **Distinct purposes get distinct streams**, never sequential draws from one
//!    stream. Streams are free (they are folded into the key); sequence position is not.
//!
//! Violating either turns replay from O(1) into O(re-simulate the trajectory), which
//! is exactly the cost the project exists to avoid. See `docs/BUDGET.md`.
//!
//! # Address packing
//!
//! Philox4x32 has a 64-bit key and a 128-bit counter. We need 64 (step) + 64 (entity)
//! + arbitrary (stream) bits of address, which overflows the counter. So:
//!
//! - counter = `[step_lo, step_hi, entity_lo, entity_hi]`
//! - key     = `splitmix64(seed ^ splitmix64(stream))`
//!
//! Streams are mixed into the key rather than the counter, so the stream space is
//! unbounded. Sub-draws beyond the 4 words per address use derived streams
//! (`Stream::sub(n)`), never counter increments.

/// Philox4x32 multipliers and Weyl constants (Salmon et al., SC'11, Random123).
const M0: u32 = 0xD251_1F53;
const M1: u32 = 0xCD9E_8D57;
const W0: u32 = 0x9E37_79B9;
const W1: u32 = 0xBB67_AE85;

/// Entity address used for events that belong to no single entity (global SSA draws).
pub const ENTITY_GLOBAL: u64 = u64::MAX;

/// A named RNG stream. Distinct purposes MUST use distinct streams.
///
/// The `u64` is folded into the Philox key, so the stream space is unbounded and
/// there is no correlation cost to spending streams liberally. Spend them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stream(pub u64);

impl Stream {
    /// Reaction-channel selection in the SSA.
    pub const SSA_CHOICE: Stream = Stream(0x0000_0000_0000_0001);
    /// Exponential waiting-time draw in the SSA.
    pub const SSA_TIME: Stream = Stream(0x0000_0000_0000_0002);
    /// Brownian displacement (P2+).
    pub const DIFFUSION: Stream = Stream(0x0000_0000_0000_0003);
    /// Binding-attempt acceptance (P3+).
    pub const BIND: Stream = Stream(0x0000_0000_0000_0004);
    /// Choosing *which* individuated molecule of a species participates.
    ///
    /// This stream exists only in the individuated arm. Because draws are addressed
    /// rather than sequential, spending it does not shift any other stream — which is
    /// what lets the counted and individuated arms produce bit-identical count
    /// trajectories from one seed. With a sequential generator they would diverge on
    /// the first reaction, and the two-arm comparison in `docs/PREREGISTRATION.md`
    /// would be impossible to run.
    pub const ENTITY_PICK: Stream = Stream(0x0000_0000_0000_0005);

    /// Derive a sub-stream for draws beyond the 4 words available at one address.
    ///
    /// Use this instead of incrementing the counter: the counter encodes *address*
    /// (step, entity) and must stay meaningful for standalone replay.
    #[inline]
    pub const fn sub(self, n: u64) -> Stream {
        // Odd multiplier keeps sub-streams distinct from base streams of other purposes.
        Stream(self.0.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(n))
    }
}

/// SplitMix64 finalizer — used only to derive keys from (seed, stream), never for draws.
#[inline]
const fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
const fn mulhilo32(a: u32, b: u32) -> (u32, u32) {
    let p = (a as u64) * (b as u64);
    ((p >> 32) as u32, p as u32)
}

/// Raw Philox4x32-R bijection. `r` is the round count; 10 is the standard.
#[inline]
pub const fn philox4x32(mut ctr: [u32; 4], mut key: [u32; 2], r: usize) -> [u32; 4] {
    let mut i = 0;
    while i < r {
        if i > 0 {
            key[0] = key[0].wrapping_add(W0);
            key[1] = key[1].wrapping_add(W1);
        }
        let (hi0, lo0) = mulhilo32(M0, ctr[0]);
        let (hi1, lo1) = mulhilo32(M1, ctr[2]);
        ctr = [hi1 ^ ctr[1] ^ key[0], lo1, hi0 ^ ctr[3] ^ key[1], lo0];
        i += 1;
    }
    ctr
}

/// The project's RNG. Holds only the seed — it has no mutable state by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    seed: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng { seed }
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The four raw words at an address. Everything else is derived from this.
    #[inline]
    pub fn words(&self, stream: Stream, step: u64, entity: u64) -> [u32; 4] {
        let k = splitmix64(self.seed ^ splitmix64(stream.0));
        let key = [k as u32, (k >> 32) as u32];
        let ctr = [
            step as u32,
            (step >> 32) as u32,
            entity as u32,
            (entity >> 32) as u32,
        ];
        philox4x32(ctr, key, 10)
    }

    /// Uniform on [0,1), from word `idx` (0..4) at this address.
    ///
    /// Open at 1.0 by construction: 24-bit mantissa fill, max value 1 - 2^-24.
    #[inline]
    pub fn uniform(&self, stream: Stream, step: u64, entity: u64, idx: usize) -> f64 {
        let w = self.words(stream, step, entity);
        u32_to_unit(w[idx & 3])
    }

    /// Exponential with unit rate, from word `idx`. Guarded against `ln(0)`.
    #[inline]
    pub fn exponential(&self, stream: Stream, step: u64, entity: u64, idx: usize) -> f64 {
        // 1 - u is in (0,1], so ln is finite and the result is in [0, inf).
        -(1.0 - self.uniform(stream, step, entity, idx)).ln()
    }
}

/// Map a u32 to [0,1) using the top 24 bits — exactly representable in f64, never 1.0.
#[inline]
pub fn u32_to_unit(w: u32) -> f64 {
    ((w >> 8) as f64) * (1.0 / 16_777_216.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-answer tests from Random123's `kat_vectors` for philox4x32 with 10 rounds.
    ///
    /// This is the load-bearing test for invariant 7: if our Philox does not match the
    /// reference bit-for-bit, "same seed => bit-identical trajectory" is a claim about
    /// our bugs rather than about the algorithm.
    #[test]
    fn philox4x32_10_known_answer_vectors() {
        assert_eq!(
            philox4x32([0, 0, 0, 0], [0, 0], 10),
            [0x6627_e8d5, 0xe169_c58d, 0xbc57_ac4c, 0x9b00_dbd8]
        );
        assert_eq!(
            philox4x32(
                [0xffff_ffff, 0xffff_ffff, 0xffff_ffff, 0xffff_ffff],
                [0xffff_ffff, 0xffff_ffff],
                10
            ),
            [0x408f_276d, 0x41c8_3b0e, 0xa20b_c7c6, 0x6d54_51fd]
        );
        assert_eq!(
            philox4x32(
                [0x243f_6a88, 0x85a3_08d3, 0x1319_8a2e, 0x0370_7344],
                [0xa409_3822, 0x299f_31d0],
                10
            ),
            [0xd16c_fe09, 0x94fd_cceb, 0x5001_e420, 0x2412_6ea1]
        );
    }

    #[test]
    fn draws_are_pure_functions_of_their_address() {
        let rng = Rng::new(0xDEAD_BEEF);
        // Same address, called in a different order, out of sequence, twice.
        let a = rng.uniform(Stream::SSA_TIME, 12345, 7, 2);
        let _ = rng.uniform(Stream::DIFFUSION, 999, 3, 0);
        let _ = rng.uniform(Stream::SSA_TIME, 12346, 7, 2);
        let b = rng.uniform(Stream::SSA_TIME, 12345, 7, 2);
        assert_eq!(a, b, "an address must always yield the same bits");
    }

    #[test]
    fn streams_step_and_entity_all_decorrelate() {
        let rng = Rng::new(1);
        let base = rng.words(Stream::SSA_CHOICE, 100, 5);
        assert_ne!(base, rng.words(Stream::SSA_TIME, 100, 5), "stream must matter");
        assert_ne!(base, rng.words(Stream::SSA_CHOICE, 101, 5), "step must matter");
        assert_ne!(base, rng.words(Stream::SSA_CHOICE, 100, 6), "entity must matter");
        assert_ne!(
            base,
            Rng::new(2).words(Stream::SSA_CHOICE, 100, 5),
            "seed must matter"
        );
    }

    #[test]
    fn sub_streams_are_distinct_and_unbounded() {
        let rng = Rng::new(42);
        let mut seen = std::collections::BTreeSet::new();
        for n in 0..64u64 {
            let w = rng.words(Stream::BIND.sub(n), 0, 0);
            assert!(seen.insert(w), "sub-stream {n} collided");
        }
    }

    #[test]
    fn uniform_is_in_half_open_unit_interval() {
        assert_eq!(u32_to_unit(0), 0.0);
        let max = u32_to_unit(u32::MAX);
        assert!(max < 1.0, "uniform must be open at 1.0, got {max}");
        assert!(max > 1.0 - 1e-7);
    }

    /// A weak smoke test on the first two moments. This is *not* a substitute for
    /// the L0 statistical ladder in `protocell-validate` — it only catches a wiring
    /// error that would make draws grossly non-uniform.
    #[test]
    fn uniform_moments_are_sane() {
        let rng = Rng::new(7);
        let n = 200_000u64;
        let mut sum = 0.0;
        let mut sumsq = 0.0;
        for i in 0..n {
            let u = rng.uniform(Stream::SSA_CHOICE, i, 0, 0);
            sum += u;
            sumsq += u * u;
        }
        let mean = sum / n as f64;
        let var = sumsq / n as f64 - mean * mean;
        assert!((mean - 0.5).abs() < 0.01, "mean {mean}");
        assert!((var - 1.0 / 12.0).abs() < 0.01, "var {var}");
    }
}
