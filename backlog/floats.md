# Step 101: floats

T has no floating-point types. They're overdue for ML, BLAS and graphics, and they're the first
stop on the ML/GPU path below. Release 0.109.0.

## Decided (2026-11-27, all as recommended)

- **D1 `Eq`/`Ord`:** `f32` and `f64` are neither. `== != < <= > >=` on floats are IEEE's
  comparisons, built in as integer `<` is (NaN compares false, `!=` true; `0.0 == -0.0`), so
  `Eq` and `Ord` keep their laws, `derive(Eq)` of a struct with a float is refused, and
  `total_cmp` gives an `Ordering` to sort by. Posits, if they come, would be lawfully `Eq` and
  `Ord` (one NaR, compare as integers): the decision is per type, so `Float` means IEEE binary
  floats and a shared supertrait can sit above both then.
- **D2 `as`:** the same number, or a panic, for every pair of numeric types. Never panics where
  every value of the source fits exactly (`i32 as f64`, `f32 as f64`, `u8 as f32`); from a
  wider integer, doesn't where the logic proves `|x| <= 2^53` (`2^24` for `f32`); float to
  integer may always panic (NaN, a fraction, out of range). Rounding is named: `nearest`, and
  `x.round() as i64`.
- **D3 literals, like integers:** `1.0`, `2.5e-3`, `1e10`, `2.5f32`, `1f64`; inferred, no
  default; `1` is never a float; no `.5`, `1.` or hex floats; a literal that rounds to
  infinity is an error. Rounded once, from the decimal to its final type.
- **D4 refinements:** floats aren't in the logic. A predicate that names a float is refused,
  and nothing is learned from a float comparison; generic predicates still hold at `T = f64`,
  comparing bits (the model of docs/85).

## The rest of the shape (Claude's calls)

- `+ - * /`, unary `-`: IEEE, round to nearest even, never panic, so no effect. No `%`.
- NaN in, NaN out, everywhere, `min`/`max` too (IEEE 754-2019 `minimum`/`maximum`). The sign
  and payload of a NaN that arithmetic makes are unspecified, as in Rust (RFC 3514).
- The same results everywhere: interpreter, const eval and C all round each operation once;
  C gets `-ffp-contract=off -fno-math-errno`, never fast-math; literals go to C as exact hex.
  Fast-math is out: "no NaN" is UB that can delete the range check of `x as usize`, and
  reassociation makes const eval, the interpreter and C disagree.
- Sealed `Float` (like `Int`): operators, literals and comparisons on `F: Float`; its methods
  work on `f64` and on `F`: `abs`, `copysign`, `floor`, `ceil`, `trunc`, `round`, `sqrt` (the
  one new trusted operation, correctly rounded by IEEE), `min`, `max`, `is_nan`,
  `is_infinite`, `is_finite`, `is_sign_negative`, `total_cmp`; `to_bits`/`from_bits` per type.
- `Display`/`Debug`/`Parse` written in T, in core, on exact big numbers in fixed arrays (no
  alloc, no panic): the shortest decimal that parses back to the same bits, laid out as
  Rust's `{:?}` (`0.1`, `1.0`, `-0.0`, `1e16`, `1.5e-7`, `inf`, `NaN`); parsing rounds
  correctly; `f"{x:.3}"` rounds the exact value. Interpreter and C run the same code.
- `extern fn` takes and gives `f32`/`f64` as C `float`/`double`; hosted builds link libm.
- Not in: float patterns, atomic floats, `%`, `mul_add`, math functions (step 102).

## After floats

Interest raised 2026-11-26 (ML/AI, reinforcement learning, GPUs; inspired by Bend). The
machine has an NVIDIA RTX 5070 (12 GB).

Not Bend's model: running arbitrary code on the GPU needs a runtime like HVM2's
interaction nets, which goes against having no runtime. Instead, a *checked kernel
subset*: a kernel is ordinary T `with {}` (effects already exclude alloc, panic and io),
refinements prove the indices (no bounds checks on the device), and disjoint per-thread
writes are a typing rule.

1. Floats (this step).
2. Math functions in T (`exp`, `ln`, `pow`, `sin`, `cos`, `tanh`), the same on every
   machine, which libm isn't.
3. A small CPU matrix/MLP library in t-libs, and an RL agent that learns a T game
   (t-maze, for example) as dogfood.
4. FFI with pointers, plus linking shared libraries (also unlocks graphics, windowing and
   sqlite).
5. `std::gpu` over the CUDA driver API (libcuda: `cuMemAlloc`, `cuModuleLoadData`,
   `cuLaunchKernel`).
6. `#[kernel]` fns compiled by the C backend to CUDA C, then NVRTC, then PTX.

The GPU comes later, once the CPU version of the RL agent is the bottleneck.
