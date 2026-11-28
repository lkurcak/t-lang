# `Fn` bounds leave a closure's effects open

Decided by the owner on 28 Nov 2026, from "Generic helpers over closures must spell out every
effect" in t-fmt's `FRICTION.md`. When the step is done, this goes under its doc's
`## Decisions`, and this file is deleted.

## The decision

An `Fn`, `FnMut` or `FnOnce` bound that says no `with` leaves what its calls may do open, as
`I: Iterator` leaves `I::e` open: a bound constrains only what it says. What its calls may do
is named `F::e`.

- `F: Fn(i64) -> i64 with {}` says its calls do nothing. `with X` says at most `X`, as
  today.
- A function whose effects are inferred (not exported, no `with`) has `F::e` among them where
  it calls an `F`, with nothing written:

  ```t
  fn twice<T: Discard, F: FnMut() -> T + Discard>(mut f: F) -> (T, T) { (f(), f()) }
  ```

- An exported function says it: `export fn map<U, F: FnOnce(T) -> U + Discard>(self, f: F)
  -> Option<U> with F::e`.
- `effect e` stays, for one name over several bounds, or for a function pointer's type.
- A function pointer's type, `fn(i64) -> i64`, is a type, not a bound: there, no `with`
  still means none.

Rejected:

- An effect inferred only for bounds in functions whose effects are inferred. The same bound
  would read two ways, depending on `export` and on whether the function writes a `with`.
- Today's reading, with better messages. Every real `Fn` bound in core, alloc, std, t-libs,
  t-fmt and t-life names an effect parameter. None relies on a bound with no `with` meaning
  none.

## What was found (0.110.0)

- Half of the friction is gone already: a private helper with `effect e` and `F: .. with e`
  that writes no `with` of its own infers `{alloc, panic, e}`. t-fmt's `comma_list` lost
  inference only by writing `with e`. The message for that, ``help: say `with {alloc, e}` ``,
  should also offer dropping the `with` when the function isn't exported.
- When a closure doesn't fit an effect-free bound, the message prints a refinement condition
  (`-1 <= n && n <= 4294967294`) and a note about lending, but doesn't name the bound or the
  fix. With `with {}` bounds, it should say which bound allows no effects.
- Today's reading is in `BoundItem::effects` (`src/ast.rs`: "With `sig` but without this,
  they can do nothing") and in `preds` (`src/check/traits/bounds.rs`). `preds` makes
  `<F as FnOnce>::Sig` equal a function type with the written effects, or none.
- In `tests/`, 165 `F:` bounds say no `with`. Nearly all pass closures that do nothing, and
  keep working. The tests of a refusal, such as `err_soundness_effect_closure_bound.t`, will
  say `with {}`.

## For the plan

- The cheapest sound way: a bound with no `with` gets an effect parameter of its own, and
  `F::e` names it. Each use decides it from the argument's `Sig`, as
  `impl<U, effect fx, I: Iterator, F: FnMut(I::Item) -> U with fx>` already does. E1–E5 of
  `docs/24-visibility.md` then hold, as for a written `effect e`.
- `F::e` is ambiguous when `F` is both an `Fn` and an `Iterator`, as an associated type of two
  traits is: write `<F as FnOnce>::e`.
- An opaque result, `-> impl Fn(u8) -> u8`, follows `impl Iterator<Item = u8>`: open, unless
  a `with` bounds it. Check this against `docs/68.1-effect-bindings.md`'s reasons.
- A trait method's open bound, inherited by an impl that says `with {}` there, asks more of
  the argument than the trait does, so it's refused.
- Migrate core, alloc and std: an `effect e` used by only one `Fn` bound becomes `F::e`. Then
  t-libs, t-fmt (its helpers drop `effect e` and their own `with`), and the docs:
  `docs/reference.md`'s closures and effects, and `docs/06c-closures.md`'s examples. Mark
  t-fmt's `FRICTION.md` entry as fixed.
- Families: the signature family, over where a bound is (a function, an impl, a trait's
  method, an opaque result), with or without a `with`, exported or inferred, each run with
  the interpreter's oracle. Plus the inheritance family, for a trait method's bound kept by
  an impl.
