# Backlog

## Now

1. Owner: decide how a generic helper that takes a closure says its effects. Today it spells
   out every one (`effect e`, `F: .. with e + Discard`, then `with {alloc, panic, e}` in
   full). In a private function, either the rest is inferred beside a written `e`, or an `F:
   FnMut(..)` bound that names no effect gets an inferred one (see "Generic helpers over
   closures" in t-fmt's `FRICTION.md`).
1. Checking a program costs more as core grows: each check clones the libraries' checker and
   their parsed program, about 15 ms more per program in the test profile since step 102's
   2,700 lines, so CI's Ubuntu test job went from 19 to 30 minutes and macOS's from 6.5 to 17.
   Share them (`Arc`, copy on write) instead of cloning, before step 103 adds more to core.
1. Step 103, the rest of the math functions, correctly rounded on step 102's machinery
   (`docs/102-math.md`): `asin`, `acos`, `atan`, `atan2`, `asinh`, `acosh`, `atanh`, `cbrt`
   (glibc's `cbrt(-27.0)` is `-3.0000000000000004`) and `hypot`, then `sin_pi` and `cos_pi`;
   then the [ML and GPU path](backlog/ml-path.md).

## Later

- [Checking faster](backlog/compile-time.md): a cache of each package's check on disk, then
  function bodies checked in parallel. Due at the first program over about 10,000 lines or a
  check over a few seconds, and t-fmt is 9,600 lines that take 6 seconds to check.
- [Checking time still doubles](backlog/joins-by-reference.md) with each `if let` in a row
  that sets the same variable (fourteen take 15 seconds): where branches meet, name a
  branch's value rather than copy what's known of it (#150; "What it doesn't do" in
  `docs/61-proof-search.md`). Due once a function takes over about 2 seconds to check;
  t-fmt's slowest takes half a second. A generated test's loop with forty `if`s in a row,
  each setting the same variable, took 74 seconds.
- Assigning through a call's result: `module(p, i).doc = x`, where `module` gives a `&mut`
  (see "Can't assign through a call's result" in t-fmt's `FRICTION.md`).
- `usize::MAX`: today it's `core::num::greatest::<usize>()`, which is hard to find. Make it
  findable, or add associated consts.
- `join` and `map().collect()` in the libraries: t-fmt wrote both by hand.
- `t fmt` never breaks a long chain of method calls, so it lays one out badly. Any change to
  `t fmt` is ported to t-fmt in the same change.
- Keywords that are ordinary words (`sealed`, `law`, `ensures`, `invariant`, `effect`,
  `with`) as names wherever they can't be read as the keyword. t-fmt's `FRICTION.md` says
  they bite constantly; step 100 only makes the message say it's a keyword.
- [The ML and GPU path](backlog/ml-path.md), after math functions: CPU float speed measured
  against Rust and C and then tuned; a matrix and neural-network library in t-libs, tested on
  MNIST from a linear model to a small residual net; an agent that learns to play a T game
  such as t-maze; then FFI with pointers and linking shared libraries; GPU kernels later.
- Floats, what step 101 left out (`docs/101-floats.md`): refinements of floats (measures like
  `is_finite` first, so `{ x: f64 | x.is_finite() }`), `%` (exact, as C's `fmod`), `mul_add`,
  hex float literals, a wrapper that's `Eq` and `Ord` by `total_cmp`, atomics of floats, and
  faster text (Ryu to write, Eisel-Lemire to read past the fast path: today reading 17 digits
  takes the interpreter about 160 ms). A posit type, if machines get them, would be `Eq` and
  `Ord`, beside `Float` under a shared supertrait.
- The logic can't bound `2 * i * (2 * i + 1)` from `i < 1000`, a product of two linear terms
  in one variable, though it bounds `i * i`; and the message's "known here" leaves out the
  range it did know. Found in step 102 (`sincos` in `core/float/fix.t` works around it).
- Checking long functions: a `main` of 4,129 lines of `show(&mut out, "..")?;` took 38 GB and
  over five minutes before it was stopped, and an array literal's check grows faster than its
  length (2,000 elements 0.4 s, 4,000 1 s, 11,000 16 s). Found writing step 101's generated
  tests, which now make their values in T instead.
- Fast floats, explicitly and the same everywhere: `mul_add` (one rounding; slow without an
  FMA unit), and sums and dot products defined in a SIMD-shaped order (eight accumulators
  joined in a fixed tree), so they vectorise without fast-math. Never "assume no NaN", which
  is UB in C (it can delete the range check of `x as usize`).
- `chars`, the other iterators of a `str`, and ranges as bounded iterators, so
  `s.chars().enumerate()` can't panic (see "What it doesn't do" in
  `docs/92-where-a-bound-holds.md`).
- Laws for `count`: at most what a bounded iterator has left, and the `len` of an exact one
  (the same section).
- An effect a function has unless a bound holds, `with panic unless T: Small`: a closure in
  its body has its effects whatever the bound, an impl's own method in place of such a default
  is refused the effect rather than checked under the bound, and there's one bound, of a type
  parameter (not `Skip<I>`); see "What it doesn't do" in `docs/92-where-a-bound-holds.md`.
- An invariant where a bound holds (`docs/93-invariants-where.md`): a field's refinement with
  one part that always holds beside one that hangs on a bound, and the same on a variant's
  field or a parameter.
- Second walks only where they're needed: a body that only reads a struct with a conditional
  invariant needn't be walked again under the bound (perf; needs a sound split of reads from
  writes).
- Laws checked at an instance: an impl of the stronger trait for fewer types than the weaker
  one's (`Range<usize>: ExactSizeIterator`) needs the generic body walked at that type.
- More iterator laws: `zip`'s length (needs `J: ExactSizeIterator` in the law), `&mut I` as
  an `ExactSizeIterator` (measures through `&mut` are refused, `docs/86-trait-contracts.md`),
  laws of derived and built-in `Eq` (derive would write bodies that can be checked), and
  blanket impls of a law's trait, refused today whenever any impl of the other trait unifies.
- Conditions beside an effect parameter or associated effect (`with {e, panic if ..}`,
  refused today; a call would keep the rest of the callee's effects and drop only the
  condition's), and conditions on trait methods, inherited as contracts are.
- A `panic(..)` in a branch the facts rule out still counts as an effect unless the walk
  knows the branch is dead; excusing it needs a proof of `false` per site (measure the cost
  first).
- Closures never meet blanket impls over `Fn` bounds (`impl<F: Fn() -> u8> B for F`), once a
  use appears.
- Two restrictions #148 names: a projection of a type with a concrete head in a trait
  method's signature or a struct field doesn't see that item's bounds (`<W<T> as Go>::e`
  says "`W<T>` doesn't implement `Go`"), and `Self::e` in a `let`'s type in an impl's method
  says "unknown type `Self`".
- One copy of the program's impls: the checker's `Projections` keeps its own, kept equal to
  `Traits` by hand and an assertion (and `info.traits` for monomorphisation).
- A lifetime error names a lifetime implied by the one needed, not that one: `exprs(p: &mut
  Parser, close: Tok)` calling `expect(p, close)` says `close` must outlive `Parser`'s first
  lifetime, which follows from outliving its second, the one `expect` asks for.
- Patterns, once a use appears: consts of structs and enums (matching by structure), ranges of
  consts (`LO..=HI`), and `..` in slice patterns, which don't exist yet (see "What it doesn't
  do" in `docs/100-patterns.md`).
- An array's pattern forgets what its elements' type says: `let [_, y] = a`, where `a: [{ v:
  u8 | v < 3 }; 2]`, gives a `y` not known to be below 3, though `a[1]` is, and so is what a
  tuple's pattern takes out.
- A type alias as a path in an expression: `type F = f64;` then `F::from_bits(b)` says an
  alias "can only be used as a type". Natural in float code written for one type.
- Messages: `greatest::<T>()` of a type parameter says "unknown type `T`"; `old(..)` deeper
  than what's kept says "could not be represented by the supported reasoning" instead of
  that what a parameter pointed to that deep isn't known; a scenario under nested
  projections lists every implied bound, eight times; the orphan error for `impl<T> lib::Tr
  for T` says "`T` is `core`'s"; a pattern in a `for` that can fail to match, `for (C, x)
  in a`, says "non-exhaustive `match`: `Option::Some(_)` not covered".
- `IntoIterator` for t-libs' `Grid` and `HashMap` and std's `Args`, so `for` takes them.
- Doctests: the code in doc comments checked and run (`README.md`, Tests, calls them the
  natural next step).
- The test suite's speed, once the libraries are shared: profile what the compiled and golden
  tests spend (888 s of CPU on 4 cores) and the unit tests (3 minutes on Ubuntu, 6 on
  Windows); then maybe tests split into those for every platform and those for the home
  server only (a forge decision first). `scripts/ci_times.py` shows where a run's time goes.
- Homebrew: `brew upgrade t` says there's no formula named `t`; check the tap's formula name.
- A look at the process for every project, in the owner's `CLAUDE.md`, with forge reporting
  CI times across repos, if t-lang's (each retrospective, in its `CLAUDE.md`) pays.
- Not decided, the owner's ideas for new programs: an embedded database (a B-tree over
  pages), an HTTP server (after an `Os` networking step), a graphical game (after FFI with
  pointers), compute shaders, a progress-bar library (once a T CLI needs one).
- Writing the compiler in T: not yet. First months without breaking changes to the core,
  checking about 10 times faster (the compiler is about 84,000 lines of Rust), and a seed in
  generated C to bootstrap from; t-fmt was the trial run, and it went well.

## Done

- A first look at the process: the 32 generated families in one test binary, so they run
  side by side (575 s to 247 s on four cores), sanitizer runs that cancel older ones,
  `scripts/ci_times.py`, and a look at the process at each retrospective (#163).
- Step 102 (0.110.0): math functions of `f32` and `f64`, correctly rounded and written in T:
  `exp`, `exp2`, `exp_m1`, `ln`, `log2`, `log10`, `ln_1p`, `pow`, `sin`, `cos`, `tan`, `sinh`,
  `cosh`, `tanh`, checked against MPFR on every `f32` input; consts read in place; a refined
  `let` kept after a loop's `break`; `t check` types less of the libraries (#161).
- Retrospective 20 (0.109.1): no soundness hole in 0.108.0's patterns or 0.109.0's floats;
  `t check` of an empty program back to 117 ms from 180, as library functions say their
  effects; `..` in a measure's `match`, and `f32 as F`, accepted; generated tests for floats in
  every place a value can be and for what's learned on one path of the new patterns (#160).
- Step 101 (0.109.0): floats, `f32` and `f64`: IEEE 754's arithmetic, literals, comparisons
  (neither `Eq` nor `Ord`), `as` the same number or a panic, a sealed `Float`, and `Display`,
  `Parse` and `{x:.3}` written in T on exact big numbers, the same in the interpreter and C.
- Step 100 (0.108.0): let chains in `if` and `while`, `..` in variant, tuple and array
  patterns, consts and const parameters as patterns, and clearer messages for a keyword used
  as a name, a parameter called as a function, and which lifetime an error means; t-fmt takes
  the new syntax.
- t-fmt: `t fmt` written in T, about 9,600 lines, printing byte for byte what `t fmt` does
  for every T file in this repo's tests, examples and libraries; its CI compares the two.
- Step 99 (0.107.3, #151): a refinement proved of a value in one branch is no longer known in
  the other branch or after the branches meet (a soundness hole since 0.77.0).
- Step 98 (0.107.2, #150): checking time no longer doubles with each level of nested `if`s
  and `match`es. A function of t-fmt took 28 seconds to check, now a quarter of one.
- Step 97 (0.107.1, #149): a bound an associated effect hangs on is decided with every impl,
  derived ones too, the same in every package (a hole since 0.107.0); three crashes fixed.
