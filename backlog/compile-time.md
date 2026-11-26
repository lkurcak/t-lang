# Compile time

Baseline (2026-11-22, `t` 0.106.0), check and build times:

| program | lines | check | build |
|---|---|---|---|
| tcalc | 566 | 0.20 s | 0.86 s |
| t-chess | 946 | 0.43 s | 1.20 s |
| t-maze | 1052 | 0.66 s | 1.78 s |
| t-life | 1163 | 0.40 s | 1.60 s |
| t-nonograms | 1798 | 0.67 s | 2.17 s |
| t-fmt (0.107.3) | ~9.6k | 6.2 s | 18 s |

That's ≈2–3k lines/s checked; a build spends about half its time on C generation and half
in `cc`. Estimated (not measured): 10–30× slower per line than `cargo check`.

The plan, in order:

1. a per-package check cache on disk (libraries and dependencies checked once);
2. checking fn bodies in parallel. Proofs are per fn, against signatures only.

Trigger: the first program over ~10k lines, or a check over a few seconds. t-fmt is
close on both. Related: the Q4 work (typed-only library bodies, 0.94.2) brought an empty
`t check` from 285 ms down to 88 ms.
