# Backlog

## Now

1. A batch for patterns: `..` in tuple and tuple-variant patterns (`ExprKind::If(..)`), consts
   as patterns, and `&&` chains in `while let`; with clearer messages for a keyword used as a
   name, a parameter that shadows a function, and which lifetime an error is about. All found
   by t-fmt (see its `FRICTION.md`), whose own parser and printer take the new syntax in the
   same change.
2. Floats (`f32`, `f64`), as a design step first. Owner to decide: `Eq` and `Ord` against
   NaN, whether a float can be refined (not at first, is the proposal), literals, formatting
   and parsing, the C backend.
3. Owner to decide: a generic helper that takes a closure has to spell out every effect
   (`effect e`, `F: .. with e + Discard`, then `with {alloc, panic, e}` in full). In a private
   function, either the rest is inferred beside a written `e`, or an `F: FnMut(..)` bound that
   names no effect gets an inferred one (see "Generic helpers over closures" in t-fmt's
   `FRICTION.md`).
4. A retrospective after the next feature release: look for soundness holes in what shipped
   since the last one, with generated tests for a fact learned on one path and read on
   another (the hole 0.107.3 closed).

## Later

- Checking time still doubles with each `if let` in a row that sets the same variable
  (fourteen take 15 seconds): where branches meet, name a branch's value rather than copy
  what's known of it (#150; "What it doesn't do" in `docs/61-proof-search.md`). Due once a
  function takes over about 2 seconds to check; t-fmt's slowest takes half a second.
- Assigning through a call's result: `module(p, i).doc = x`, where `module` gives a `&mut`
  (see "Can't assign through a call's result" in t-fmt's `FRICTION.md`).
- `usize::MAX`: today it's `core::num::greatest::<usize>()`, which is hard to find. Make it
  findable, or add associated consts.
- `join` and `map().collect()` in the libraries: t-fmt wrote both by hand.
- `t fmt` never breaks a long chain of method calls, so it lays one out badly. Any change to
  `t fmt` is ported to t-fmt in the same change.
- After floats: a matrix and small neural-network library in t-libs, and an agent that learns
  to play a T game such as t-maze; then FFI with pointers and linking shared libraries; GPU
  kernels later.
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
