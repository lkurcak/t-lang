# Backlog

## Now

1. [Step 100: patterns batch](backlog/patterns-batch.md): `..` in tuple and tuple-variant patterns, consts as patterns, `&&` chains in `while let`, three messages; t-fmt follows in the same change.
2. [Step 101: floats](backlog/floats.md): `f32`/`f64` (a design step: `Eq`/`Ord` vs NaN, literals, formatting and parsing, the C backend); the first stop on the ML/GPU path.
3. [Effect parameters for closure helpers](backlog/effect-param-ergonomics.md): needs a decision. A generic helper that takes a closure has to spell out `effect e`, `Discard` bounds and every effect of its own.
4. Retrospective 20, after the next feature release. Add step 99's axis: something learned under one path and read on another (the sibling branch, after the join, the next iteration, a closure's body, a branch that can't be taken).

## Later

- [Joins by reference](backlog/joins-by-reference.md): sequential `if`s that set one `Option` still double the join tree with each statement. Do it as soon as any function takes over ~2 s to check.
- [Compile time](backlog/compile-time.md): a per-package check cache on disk, then checking fn bodies in parallel. Trigger: the first program over ~10k lines, or a check that takes a few seconds (t-fmt: 9.6k lines, 6.2 s).
- Assign through a call's result: `module(p, i).doc = ..` where `module` returns `&mut`.
- `usize::MAX`: today there's only `core::num::greatest::<usize>()`. Needs associated consts or a name people will find.
- `join` on lists of strings, and `map(..).collect()`.
- `t fmt` breaks long method chains (`.add()` chains) badly; port any change to t-fmt in the same change.
- `IntoIterator` for t-libs `Grid`/`HashMap` and std `Args`.
- Law checks at an instantiation (an S impl for fewer types than P's, `Range<usize>: ExactSizeIterator`, `Range<usize>` bounded): walk a generic body at a substitution.
- A constructor law for `zip` (needs `J: ExactSizeIterator` in the law).
- `&mut I: ExactSizeIterator`: measures through `&mut` are refused (docs/86, H1).
- Laws of glue traits' methods (derived or built-in `Eq`): derive writes bodies that can be checked.
- Blanket impls of a law trait are over-refused whenever any P impl unifies.
- Conditions on trait methods (inherited by entailment; needs an inheritance family), and conditions beside effect parameters or associated effects: a site keeps the callee's unconditional effects, instantiated, and drops only what the conditional atom put there (store the unconditional part per `Site`). Today this is refused outright.
- Dead unconditional sites: a `panic(..)` in a branch the facts refute still counts unless the walk's `reachable` is false. Excusing it needs a `Φ ⊢ false` proof per site; measure the cost first.
- `Chars` and the other str iterators as `BoundedIterator`s (their `next` needs its own invariant).
- `count` laws `r <= old most` / `r == len`.
- Calls inside closures keep guarded effects (`Within` has no callee or type args).
- Impl overrides of a guarded method: walk them under `Head: B` as a scenario instead of barring them.
- Guards on a partly known type (`Skip<I>: B`), and several bounds at once.
- A field refinement with one part that always holds and one that hangs on a bound.
- `where` on variant fields and params, once a use appears.
- Skip the second walk for bodies that only read (perf only; needs a sound read/write split).
- Closures never meet blanket impls over `Fn` bounds (`impl<F: Fn() -> u8> B for F`); fix once a use appears.
- A projection of a concrete-headed type in a trait method's signature or a struct field doesn't see that item's own bounds: `<W<T> as Go>::e` under `T: Val` says "`W<T>` doesn't implement `Go`".
- `Self::e` (any `Self` path) in a `let`'s type inside an impl's method says "unknown type `Self`".
- One store for impls, closures and opaque types: `Projections` keeps its own copy, synced by hand with `Traits` (and `info.traits` for mono).
- Message: `greatest::<T>()` of a type parameter says "unknown type `T`".
- Message: `old(..)` past `ENTRY_DEPTH` says "the condition could not be represented by the supported reasoning". Say instead that what a parameter pointed to that deep isn't known (a failure of its own for `Pointee::Unknown` reads).
- Message: a scenario under nested projections lists every implied bound (`Enumerate<…<I>>: BoundedIterator` eight times); name the weakest.
- Message: the orphan error for `impl<T> lib::Tr for T` says "`T` is `core`'s".
- Trait type parameters (`Borrow<Q>`, the t-libs probe idiom): not yet. Repeated parameters are unsound under resolution that ignores lifetimes, so linear heads come first, and region-aware resolution is a step of its own. `l[a..b]` waits for `Index<Idx>` and trait preconditions.
- Tests: run the test binaries concurrently and use mimalloc elsewhere too. Share the library tables through `Arc` only if clones still show up in profiles.
- Tests: profile the real CPU time of the compiled and golden tests (888 s on 4 cores); likely the better lever on CI time.
- Tests: split the suite into "every platform" (build plus a sampled C-backend smoke test) and "home server only" (everything else). Needs a forge decision first.
- Homebrew: `brew upgrade t` says there's no formula named `t`; check the tap's formula name.
- [ML/GPU path after floats](backlog/floats.md#after-floats): a CPU matrix/MLP library and an RL agent that learns a T game, FFI with pointers, `std::gpu`, `#[kernel]` fns.
- New programs, not yet decided: an embedded database (a B-tree over pages), an HTTP server (after an Os networking step), a pixel-graphics game (after FFI with pointers), a progress-bar library (when a T CLI needs one).
- Self-hosting: not yet. It needs (1) months without breaking changes in the core, (2) checking ~10× faster (the compiler is ~84k lines of Rust), (3) a generated-C seed for bootstrapping. The t-fmt port went well.

## Done

- t-fmt: `t fmt` written in T (lkurcak/t-fmt), byte-identical on t-lang's ~3000 `.t` files; its CI compares it against the pinned `t fmt`.
- Step 99: what a path learns stays on it (0.107.3).
- Step 98: joins that don't double (0.107.2).
- Step 97: bounds decided with every impl (0.107.1).
- Earlier steps: the roadmap table in [README.md](README.md) and [CHANGELOG.md](CHANGELOG.md).
