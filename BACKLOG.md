# Backlog

## Now

1. A batch for patterns (step 100): let chains in `if` and `while` (`if let Some(x) = a &&
   x > 3 { .. }`; today both refuse `&&` beside a `let`), `..` in tuple, tuple-variant and
   array patterns (`ExprKind::If(..)`), and consts as patterns; with clearer messages for a
   keyword used as a name, a parameter that shadows a function, and which lifetime an error
   is about. All found by t-fmt (see its `FRICTION.md`), whose own parser and printer take
   the new syntax in the same change.
2. [Floats](backlog/floats.md) (`f32`, `f64`), as a design step first. Owner to decide: `Eq`
   and `Ord` against NaN, whether a float can be refined (not at first, is the proposal),
   literals, formatting and parsing, the C backend.
3. Owner to decide: a generic helper that takes a closure has to spell out every effect
   (`effect e`, `F: .. with e + Discard`, then `with {alloc, panic, e}` in full). In a private
   function, either the rest is inferred beside a written `e`, or an `F: FnMut(..)` bound that
   names no effect gets an inferred one (see "Generic helpers over closures" in t-fmt's
   `FRICTION.md`).
4. A retrospective after the next feature release: look for soundness holes in what shipped
   since the last one, with generated tests for a fact learned on one path and read on
   another (the hole 0.107.3 closed).

## Later

- [Checking faster](backlog/compile-time.md): a cache of each package's check on disk, then
  function bodies checked in parallel. Due at the first program over about 10,000 lines or a
  check over a few seconds, and t-fmt is 9,600 lines that take 6 seconds to check.
- [Checking time still doubles](backlog/joins-by-reference.md) with each `if let` in a row
  that sets the same variable (fourteen take 15 seconds): where branches meet, name a
  branch's value rather than copy what's known of it (#150; "What it doesn't do" in
  `docs/61-proof-search.md`). Due once a function takes over about 2 seconds to check;
  t-fmt's slowest takes half a second.
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
- [After floats](backlog/floats.md#after-floats): a matrix and small neural-network library
  in t-libs, and an agent that learns to play a T game such as t-maze; then FFI with
  pointers and linking shared libraries; GPU kernels later.
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
- Messages: `greatest::<T>()` of a type parameter says "unknown type `T`"; `old(..)` deeper
  than what's kept says "could not be represented by the supported reasoning" instead of
  that what a parameter pointed to that deep isn't known; a scenario under nested
  projections lists every implied bound, eight times; the orphan error for `impl<T> lib::Tr
  for T` says "`T` is `core`'s".
- `IntoIterator` for t-libs' `Grid` and `HashMap` and std's `Args`, so `for` takes them.
- Doctests: the code in doc comments checked and run (`README.md`, Tests, calls them the
  natural next step).
- The test suite's speed: run the test binaries at the same time and with mimalloc
  everywhere; profile what the compiled and golden tests spend (888 s of CPU on 4 cores);
  then maybe tests split into those for every platform and those for the home server only
  (a forge decision first).
- Homebrew: `brew upgrade t` says there's no formula named `t`; check the tap's formula name.
- Not decided, the owner's ideas for new programs: an embedded database (a B-tree over
  pages), an HTTP server (after an `Os` networking step), a graphical game (after FFI with
  pointers), compute shaders, a progress-bar library (once a T CLI needs one).
- Writing the compiler in T: not yet. First months without breaking changes to the core,
  checking about 10 times faster (the compiler is about 84,000 lines of Rust), and a seed in
  generated C to bootstrap from; t-fmt was the trial run, and it went well.

## Done

- t-fmt: `t fmt` written in T, about 9,600 lines, printing byte for byte what `t fmt` does
  for every T file in this repo's tests, examples and libraries; its CI compares the two.
- Step 99 (0.107.3, #151): a refinement proved of a value in one branch is no longer known in
  the other branch or after the branches meet (a soundness hole since 0.77.0).
- Step 98 (0.107.2, #150): checking time no longer doubles with each level of nested `if`s
  and `match`es. A function of t-fmt took 28 seconds to check, now a quarter of one.
- Step 97 (0.107.1, #149): a bound an associated effect hangs on is decided with every impl,
  derived ones too, the same in every package (a hole since 0.107.0); three crashes fixed.
- Step 96 (0.107.0, #148): an impl's associated effect can hang on a bound, so `enumerate` of
  a bounded iterator can't panic.
