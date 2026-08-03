//! Statistics for the validation ladder, implemented here rather than pulled in.
//!
//! The dependency-free choice is deliberate: L0 is the level at which the *solver* is
//! checked against a closed-form answer, and a goodness-of-fit test is not a place to
//! inherit someone else's convergence criteria without reading them. Everything below
//! is standard (Lanczos log-gamma; series and continued-fraction expansions for the
//! regularised incomplete gamma) and is checked against known values in the tests.

/// Lanczos approximation to `ln Γ(z)`, g = 7, n = 9. Accurate to ~1e-13 relative.
pub fn ln_gamma(z: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_5e-7,
    ];
    if z < 0.5 {
        // Reflection: Γ(z)Γ(1−z) = π / sin(πz)
        let pi = std::f64::consts::PI;
        (pi / (pi * z).sin()).abs().ln() - ln_gamma(1.0 - z)
    } else {
        let z = z - 1.0;
        let mut x = C[0];
        for (i, c) in C.iter().enumerate().skip(1) {
            x += c / (z + i as f64);
        }
        let t = z + 7.5;
        0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
    }
}

/// Regularised lower incomplete gamma `P(a,x)`, by series expansion. Valid for x < a+1.
fn gamma_p_series(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..1_000 {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * 1e-15 {
            break;
        }
    }
    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

/// Regularised upper incomplete gamma `Q(a,x)`, by continued fraction (Lentz).
/// Valid for x >= a+1.
fn gamma_q_cf(a: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / TINY;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..1_000 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < TINY {
            d = TINY;
        }
        c = b + an / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

/// Regularised upper incomplete gamma `Q(a,x) = 1 − P(a,x)`.
pub fn gamma_q(a: f64, x: f64) -> f64 {
    assert!(a > 0.0, "gamma_q requires a > 0, got {a}");
    assert!(x >= 0.0, "gamma_q requires x >= 0, got {x}");
    if x < a + 1.0 {
        1.0 - gamma_p_series(a, x)
    } else {
        gamma_q_cf(a, x)
    }
}

/// Upper-tail p-value of the chi-square distribution.
pub fn chi_square_p_value(chi2: f64, dof: usize) -> f64 {
    assert!(dof > 0, "chi-square needs at least one degree of freedom");
    gamma_q(dof as f64 / 2.0, chi2 / 2.0)
}

/// Poisson pmf `P(k; λ)`, computed in log space so large λ does not overflow.
pub fn poisson_pmf(k: u64, lambda: f64) -> f64 {
    assert!(lambda > 0.0);
    (-lambda + k as f64 * lambda.ln() - ln_gamma(k as f64 + 1.0)).exp()
}

/// Binomial pmf `P(k; n, p)`, in log space.
pub fn binomial_pmf(k: u64, n: u64, p: f64) -> f64 {
    if k > n {
        return 0.0;
    }
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }
    let (k, n) = (k as f64, n as f64);
    let ln_c = ln_gamma(n + 1.0) - ln_gamma(k + 1.0) - ln_gamma(n - k + 1.0);
    (ln_c + k * p.ln() + (n - k) * (1.0 - p).ln()).exp()
}

#[derive(Debug, Clone)]
pub struct GofResult {
    pub chi2: f64,
    pub dof: usize,
    pub p_value: f64,
    pub bins_used: usize,
    pub samples: u64,
}

impl GofResult {
    /// Conventional non-rejection at the 1% level.
    ///
    /// The threshold is deliberately loose: this test is guarding against a *broken
    /// sampler*, not measuring an effect size, and a tight threshold on a test run in
    /// CI produces flakes that get silenced, which is worse than no test.
    pub fn passes(&self) -> bool {
        self.p_value > 0.01
    }
}

/// Chi-square goodness of fit of an observed histogram against an analytic pmf.
///
/// `observed[i]` is the count of samples equal to `i`. Bins whose expected count falls
/// below `min_expected` are merged into their neighbour from both tails inward, which is
/// what makes the chi-square approximation valid; the number of surviving bins is
/// reported so a test cannot silently degenerate to one bin and always pass.
pub fn chi_square_gof(
    observed: &[u64],
    pmf: impl Fn(u64) -> f64,
    min_expected: f64,
) -> GofResult {
    let n: u64 = observed.iter().sum();
    assert!(n > 0, "no samples");
    let nf = n as f64;

    // Expected counts, including a tail bin for everything beyond the histogram.
    let mut exp: Vec<f64> = (0..observed.len()).map(|k| pmf(k as u64) * nf).collect();
    let covered: f64 = exp.iter().sum();
    let tail = (nf - covered).max(0.0);

    let mut obs: Vec<f64> = observed.iter().map(|&x| x as f64).collect();
    obs.push(0.0);
    exp.push(tail);

    // Merge from the left until each bin clears the threshold, then from the right.
    let mut mo: Vec<f64> = Vec::new();
    let mut me: Vec<f64> = Vec::new();
    let (mut ao, mut ae) = (0.0, 0.0);
    for i in 0..exp.len() {
        ao += obs[i];
        ae += exp[i];
        if ae >= min_expected {
            mo.push(ao);
            me.push(ae);
            ao = 0.0;
            ae = 0.0;
        }
    }
    if ae > 0.0 || ao > 0.0 {
        if let (Some(lo), Some(le)) = (mo.last_mut(), me.last_mut()) {
            *lo += ao;
            *le += ae;
        } else {
            mo.push(ao);
            me.push(ae);
        }
    }

    let chi2: f64 = mo
        .iter()
        .zip(&me)
        .map(|(o, e)| {
            if *e <= 0.0 {
                0.0
            } else {
                (o - e) * (o - e) / e
            }
        })
        .sum();

    let bins_used = mo.len();
    let dof = bins_used.saturating_sub(1).max(1);
    GofResult {
        chi2,
        dof,
        p_value: chi_square_p_value(chi2, dof),
        bins_used,
        samples: n,
    }
}

/// Fisher's method for combining independent p-values: `X = −2 Σ ln p_i ~ χ²(2K)`.
///
/// Returns `(X, combined p)`.
///
/// # Why the ladder needs this
///
/// A goodness-of-fit test run at one seed has a false-failure rate equal to its
/// threshold — 1% at the 1% level, which across a suite run on every commit is a flake
/// every few weeks. The usual reaction to that flake is to try another seed until it
/// passes, which is seed-shopping: the same act as tuning a parameter until a plot looks
/// right, and `PROMPT.md` §7 forbids it.
///
/// Combining across a fixed set of seeds fixes both ends. It cannot be gamed by
/// retrying, because every seed's result is in the statistic. And it has far more power
/// against a small systematic bias — which shows up as a *drift* in the p-values that no
/// single run would reject — than any one run does.
pub fn fisher_combine(p_values: &[f64]) -> (f64, f64) {
    assert!(!p_values.is_empty(), "nothing to combine");
    let x: f64 = p_values
        .iter()
        .map(|p| -2.0 * p.max(1e-300).ln())
        .sum();
    (x, chi_square_p_value(x, 2 * p_values.len()))
}

/// Exact stationary distribution of a one-dimensional birth–death chain, from detailed
/// balance: `π(n+1)/π(n) = birth(n) / death(n+1)`.
///
/// Used for `2A ⇌ B` in a closed system, where the state is fully described by the
/// number of B and no closed form is at hand.
pub fn birth_death_chain_stationary(
    n_states: usize,
    birth: impl Fn(usize) -> f64,
    death: impl Fn(usize) -> f64,
) -> Vec<f64> {
    let mut pi = vec![0.0; n_states];
    pi[0] = 1.0;
    for n in 1..n_states {
        let d = death(n);
        if d <= 0.0 {
            break;
        }
        pi[n] = pi[n - 1] * birth(n - 1) / d;
    }
    let z: f64 = pi.iter().sum();
    assert!(z > 0.0 && z.is_finite(), "unnormalisable stationary distribution");
    for p in &mut pi {
        *p /= z;
    }
    pi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ln_gamma_matches_known_values() {
        assert!((ln_gamma(1.0)).abs() < 1e-12);
        assert!((ln_gamma(2.0)).abs() < 1e-12);
        assert!((ln_gamma(5.0) - 24.0f64.ln()).abs() < 1e-11, "Γ(5) = 4! = 24");
        assert!(
            (ln_gamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-11,
            "Γ(1/2) = √π"
        );
        assert!((ln_gamma(10.0) - 362_880.0f64.ln()).abs() < 1e-10, "Γ(10) = 9!");
    }

    #[test]
    fn chi_square_p_values_match_published_critical_points() {
        // Critical values at α = 0.05: dof 1 -> 3.841, dof 5 -> 11.070, dof 10 -> 18.307
        assert!((chi_square_p_value(3.841_46, 1) - 0.05).abs() < 1e-4);
        assert!((chi_square_p_value(11.070_5, 5) - 0.05).abs() < 1e-4);
        assert!((chi_square_p_value(18.307_0, 10) - 0.05).abs() < 1e-4);
        // ...and at α = 0.01: dof 1 -> 6.635, dof 10 -> 23.209
        assert!((chi_square_p_value(6.634_9, 1) - 0.01).abs() < 1e-4);
        assert!((chi_square_p_value(23.209_3, 10) - 0.01).abs() < 1e-4);
    }

    #[test]
    fn pmfs_are_normalised() {
        let s: f64 = (0..200).map(|k| poisson_pmf(k, 20.0)).sum();
        assert!((s - 1.0).abs() < 1e-12, "poisson sum {s}");
        let b: f64 = (0..=100).map(|k| binomial_pmf(k, 100, 0.37)).sum();
        assert!((b - 1.0).abs() < 1e-12, "binomial sum {b}");
    }

    #[test]
    fn pmf_moments_are_correct() {
        let mean: f64 = (0..300).map(|k| k as f64 * poisson_pmf(k, 42.0)).sum();
        assert!((mean - 42.0).abs() < 1e-9, "poisson mean {mean}");
        let bmean: f64 = (0..=80).map(|k| k as f64 * binomial_pmf(k, 80, 0.25)).sum();
        assert!((bmean - 20.0).abs() < 1e-9, "binomial mean {bmean}");
    }

    /// The GOF test must reject a wrong distribution, or it asserts nothing.
    #[test]
    fn gof_rejects_a_distribution_that_is_actually_wrong() {
        // Samples drawn exactly in proportion to Poisson(10), tested against Poisson(10)
        // and against Poisson(14).
        let n = 100_000.0;
        let observed: Vec<u64> = (0..40).map(|k| (poisson_pmf(k, 10.0) * n) as u64).collect();

        let right = chi_square_gof(&observed, |k| poisson_pmf(k, 10.0), 5.0);
        assert!(right.passes(), "correct hypothesis rejected: p = {}", right.p_value);
        assert!(right.bins_used > 5, "too few bins to be meaningful");

        let wrong = chi_square_gof(&observed, |k| poisson_pmf(k, 14.0), 5.0);
        assert!(
            !wrong.passes(),
            "a 40% error in λ must be rejected, got p = {}",
            wrong.p_value
        );
    }

    #[test]
    fn fisher_combines_p_values_correctly() {
        // Uniform p-values (a correct model) combine to an unremarkable p.
        let uniform: Vec<f64> = (1..=9).map(|i| i as f64 / 10.0).collect();
        let (_, p) = fisher_combine(&uniform);
        assert!(p > 0.05, "well-behaved p-values must not combine to a rejection: {p}");

        // A run of mildly small p-values — each individually survivable — combines to a
        // clear rejection. That is the power this buys over any single test.
        let drifting = vec![0.03, 0.04, 0.02, 0.05, 0.03, 0.06];
        let (_, p) = fisher_combine(&drifting);
        assert!(
            p < 0.001,
            "six p-values near 0.03 are a systematic bias, not luck: {p}"
        );

        // One unlucky seed among many must not sink an otherwise healthy set.
        let one_bad = vec![0.003, 0.4, 0.6, 0.8, 0.5, 0.3, 0.9, 0.45];
        let (_, p) = fisher_combine(&one_bad);
        assert!(p > 0.01, "a single unlucky seed must not fail the suite: {p}");
    }

    #[test]
    fn detailed_balance_recursion_reproduces_the_binomial() {
        // A <=> B with n_A + n_B = N: birth(n) = k_f (N - n), death(n) = k_r n.
        // Stationary distribution is Binomial(N, k_f/(k_f+k_r)).
        let (n, k_f, k_r) = (30usize, 2.0, 3.0);
        let pi = birth_death_chain_stationary(
            n + 1,
            |b| k_f * (n - b) as f64,
            |b| k_r * b as f64,
        );
        let p = k_f / (k_f + k_r);
        for (b, &got) in pi.iter().enumerate() {
            let exact = binomial_pmf(b as u64, n as u64, p);
            assert!(
                (got - exact).abs() < 1e-12,
                "state {b}: recursion {got} vs binomial {exact}"
            );
        }
    }
}
