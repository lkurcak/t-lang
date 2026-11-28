# Step 104: `Fn` bounds leave what calls do open

Code: `Rewrite::open_effects` in `src/check/modules.rs`, which gives each
open bound its effect parameter; `CALLS` and the `F::e` case of `binding`
in `src/traits.rs`; `preds` in `src/check/traits/bounds.rs`, for an
opaque type's and a type's bound; `hide` in `src/check/opaque.rs`;
`open_calls` in `src/check/adts.rs`; `Params::is_implicit` and
`type_args` (`src/check/paths.rs`); and `same_signature`
(`src/check/traits/impls.rs`).

A bound says only what it says. `I: Iterator` leaves what `next` does
open, `I::e`; an `Fn` bound with no `with` now leaves what its calls do
open the same way, `F::e`. So a helper that takes a closure needs no
effect parameter, and one whose effects are inferred has its closure's
with nothing written:

```rust
use alloc::boxed::Box;

fn twice<T: Discard, F: FnMut() -> T + Discard>(mut f: F) -> (T, T) { (f(), f()) }

fn quiet() -> u8 with {} {
    let (a, b) = twice(|| 2u8);
    a / b
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let (x, y) = twice(|| Box::new(20u8));
    out.line(&(*x + *y))?; // expect: 40
    out.line(&quiet())?;   // expect: 1
    Ok(())
}
```

`twice` calls `f`, so it does what `f` does: where `f` allocates, so does
that call of `twice`, and where `f` does nothing, as in `quiet`, nor does
it. Until 0.112.0 a bound with no `with` said its calls do nothing, and
every helper over closures in the libraries, in t-libs, t-fmt and t-life
named an effect parameter to say otherwise:
`fn twice<T: Discard, effect e, F: FnMut() -> T with e + Discard>`. That
still works.

## What a bound says

- `F: Fn(i64) -> i64` leaves what its calls do open: `F::e`.
- `F: Fn(i64) -> i64 with {}` says they do nothing, and `with X` that they
  do at most `X`, as before.
- A function whose effects are inferred (one that isn't exported and says
  no `with`, docs/24-visibility.md) has `F::e` among them where it calls
  an `F`. One that says its effects says `F::e` among them:

```rust
export fn apply<F: Fn(i64) -> i64 + Discard>(f: F, x: i64) -> i64 with F::e { f(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&apply(|x| x + 1, 2))?; // expect: 3
    Ok(())
}
```

```rust,error
export fn apply<F: Fn(i64) -> i64 + Discard>(f: F, x: i64) -> i64 { f(x) }
// error: `f` may have the effects `F::e`, but `apply` doesn't say `with F::e`
```

- `effect e` stays, for one name over several bounds, `F: Fn() with e, G:
  Fn() with e`, or for a function pointer's type, `fn() with e`.
- A function pointer's type is a type, not a bound: `fn(i64) -> i64`
  still does nothing.

`F::e` is short for `<F as FnOnce>::e`, what a call of an `F` does: the
effects of its signature. Where `F`'s bound says `with X`, that's `X`. If
`F` has another trait whose associated effect is `e`, `F::e` could be
either, and the checker asks which.

Inside a function, `F::e` is only itself, as an effect parameter is: it's
equal to nothing else, so it can't be used to give back something that
does more as something that does what `F` does:

```rust,error
use alloc::boxed::Box;

fn allocates() with alloc {
    let _b = Box::new(1u8);
}

fn leak<F: Fn() + Discard>(_f: F) -> fn() with F::e { allocates }

fn main() {
    let f = leak(|| {});
    f();
}
// error: expected `fn() -> () with F::e`, found `fn() -> () with alloc`
```

## Each bound, its own effect parameter

A function's, an impl's or a trait's method's bound with no `with` gets an
effect parameter of its own, named `F::e`, as if the function said
`effect F::e` and `F: .. with F::e`. A name with `::` can't be written for
a parameter, so it's never another's. It's never written in `::<..>`
either: each use decides it by what its argument's calls do, as it decides
a written one, so `twice::<u8, _>` gives `twice` its two written
parameters.

An impl's is decided by the type it's for, through the bound, as an
impl's written effect parameter is:

```rust
#[derive(Discard where F: Discard)]
struct FromFn<F> { f: F }

impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> {
    type Item = T;
    effect e = F::e;

    fn next(&mut self) -> Option<T> with F::e { (self.f)() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n = 0u8;
    let it = FromFn {
        f: || {
            if n < 3 {
                n = n + 1;
                Some(n)
            } else {
                None
            }
        },
    };
    for x in it {
        out.line(&x)?; // expect: 1
        // expect: 2
        // expect: 3
    }
    Ok(())
}
```

A trait's method's is its impls' too. An impl's method takes what the
trait's does, so where the trait's bound says no `with`, so must the
impl's, and where it says one, the impl's says the same: each is refused
otherwise, with a message that says which.

## Opaque results

`-> impl Fn(i64) -> i64` is a bound too, on the type the function
returns, and leaves what its calls do open, as `-> impl Iterator<Item =
u8>` leaves `e`. A caller sees `<impl Fn(i64) -> i64 as FnOnce>::e`, which
no `with` can name, so only a function whose effects are inferred can
call what it returns. The function can say what its calls may do instead:

```rust
fn adder(n: i64) -> impl Fn(i64) -> i64 + Discard { move |x| x + n }

fn quiet(n: i64) -> impl Fn(i64) -> i64 with {} + Discard { move |x| if x < n { n } else { x } }

fn wrap<F: Fn(i64) -> i64 + Discard>(f: F) -> impl Fn(i64) -> i64 with F::e + Discard {
    move |x| f(x)
}

fn none() -> i64 with {} {
    let q = quiet(3);
    let w = wrap(|x| if x < 0 { 0 } else { x });
    if q(1) > w(0) { 0 } else { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = adder(2);
    out.line(&a(1))?;   // expect: 3
    out.line(&none())?; // expect: 0
    Ok(())
}
```

## A type's bounds

A struct or enum can't have effect parameters, and its bound with no
`with` says nothing of its calls' effects: `Holder<F: Fn() -> i64>` holds
any closure that gives an `i64`, as a function's bound takes one. What a
call of one does is up to the function that calls it, by its own bound.

So a field can't name `F::e`: in a function that knows `F` only by its
bound, `F: Fn() with alloc`, it would be `{alloc}`, and a value made there
could then be read where `F` is a closure that does nothing:

```rust,error
struct S<F: Fn()> { f: F, h: fn() with F::e }
// error: a field can't have what calls of `F` do, `F::e`, in its type
```

## Why it's sound

**Lemma O1 (an effect parameter of its own).** A program with open bounds
means what it would with each written as `effect F::e` and `with F::e`,
which is a program of 0.111.0's language: a function's, an impl's or a
method's bound gets that parameter (`open_effects`), before anything else
is checked. So what's inferred and checked of it is what is of any
effect parameter (lemmas E1 to E5 of docs/24-visibility.md), and the
induction on the stack of docs/07-effects.md goes through as before. The
only difference is that nobody writes its type argument: each use gets a
fresh set for it, which the bound's argument decides, the least that
holds, as for a written one given `_`.

**Lemma O2 (decided where it's called).** `F::e` written in a signature
is resolved as it's written, to that parameter, or to `X` of a bound
`with X`: what the effects of `F`'s signature are, by the bounds in
scope. So no function's type holds the projection on a type parameter of
its own, and what a caller decides for `F::e` is what each copy of the
callee is given. It's never what the copy's `F` turns out to do. That
matters: in

```rust
use alloc::boxed::Box;

fn allocates() with alloc {
    let _b = Box::new(1u8);
}

fn inner<F: Fn() + Discard>(_f: F, h: fn() with F::e) { h() }

fn outer<G: Fn() with alloc + Discard>(g: G) { inner(g, allocates) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    outer(|| {});
    out.line(&1u8)?; // expect: 1
    Ok(())
}
```

`outer` decides `inner`'s `F::e` as `{alloc}`, what its `G`'s calls may
do, and gives it `allocates`. Had `F::e` been what the copy's `F` does,
`inner`'s copy for `main`'s closure would have been one whose `F::e` is
none, calling `allocates` as if it didn't allocate.

**Lemma O3 (an opaque type's).** An opaque type's bound with no `with`
says its calls do `<impl Fn() as FnOnce>::e`, of itself (`preds`), which
nothing outside its function says more of: it's rigid there, as an
opaque `impl Iterator`'s `e` is (lemma E3 of docs/68.1-effect-bindings.md).
Where it's returned, the bound is of what it stands for (`hide`), whose
calls do what they do: so it holds. Monomorphisation, which sees what it
stands for, finds those calls' effects, which are what each call of it
had in the callers' sets.

**Lemma O4 (a type's).** A type's bound with no `with` says its calls do
what they do, `<F as FnOnce>::e` of its own parameter, and at a use of the
type, that's what the argument's calls do: any closure meets it. Inside,
nothing calls an `F`: a type has no code. The fields can't say `F::e`
(`open_calls`): it would be read by each function's bounds on `F`, which
may say more than what the copy's `F` does, and a value of the type moves
between them.

**Lemma O5 (an impl's method).** An impl's method has the trait's method's
type parameters, its effect parameters among them, in the same places: so
an open bound in one is an open bound in the other, and a caller going by
the trait's gives each copy the effect argument it takes. One open where
the other says `with` is refused (`same_signature`): the trait's callers
would give a set where the impl's method has no parameter for it, or the
impl's would allow less than the trait's callers count on.

The polymorphic recursion check (docs/04-generics.md) counts an effect set
as no deeper than what's in it, as sets are flat: a function that calls
itself with its own `F`, and so its own `F::e`, `{F::e}`, makes no bigger
type. It had refused a recursive function with a written effect
parameter passed on, too.

## What it doesn't do

- **An open bound's `F::e` in a type's fields** is refused (O4). A type
  generic over what a call does would need effect parameters on types.
- **A bound in an impl or trait's method that differs from its trait's
  only by being open** is refused (O5), though one that asks less of its
  argument would do for callers: an impl's method's bounds are exactly
  its trait's.
- **Messages** name `<impl Fn(..) as FnOnce>::e` where a caller of an
  opaque result needs it, and say the function returning it can bound
  it; they don't yet offer the edit.

## Decisions

Decided by the owner on 28 Nov 2026, from "Generic helpers over closures
must spell out every effect" in t-fmt's `FRICTION.md`:

- **An `Fn`, `FnMut` or `FnOnce` bound that says no `with` leaves what its
  calls may do open**, as `I: Iterator` leaves `I::e` open: a bound
  constrains only what it says. It's named `F::e`; `with {}` says none,
  `with X` at most `X`. Rejected: inferring it only for bounds in
  functions whose effects are inferred, as the same bound would then read
  two ways, depending on `export` and on whether the function writes a
  `with`; and today's reading with better messages, as every real `Fn`
  bound in core, alloc, std, t-libs, t-fmt and t-life named an effect
  parameter, and none relied on a bound with no `with` meaning none.
- **An opaque result's bound is open too**: `-> impl Fn(u8) -> u8` with no
  `with` leaves its calls' effects open, as `impl Iterator<Item = u8>`
  leaves `e`, rather than meaning none in that one place.
- **A function pointer's type keeps meaning none**: it's a type, not a
  bound.
- **Each bound its own effect parameter, decided where it's called**,
  rather than `F::e` as what the copy's `F` does (O2's example), which
  would need `F::e` rigid and bounded in every function, and `with e`
  bounds to decide `e` differently than they do.
- **0.112.0.** A program whose `Fn` bound with no `with` was given a
  closure that may do something was refused, and is accepted; one that
  counted on such a bound to mean none says `with {}`.

## Exercises

1. Write `fn map_all<F: FnMut(u8) -> u8 + Discard>(xs: &mut [u8], f: F)`
   with no `with`, and call it with a closure that may panic and one that
   can't, from functions that say `with {}` and `with panic`. Which
   calls are refused?
2. Make `F::e` in `outer` above what the copy's `G` does, and run the
   program with `t run --interp`. What does the interpreter's check say?
3. Why can't an impl's method say `with {}` on a bound its trait's method
   leaves open? Write a caller, going by the trait, that the impl would
   be wrong for.
4. A struct `Pair<F: Fn() -> u8>` with a field `h: fn() with F::e`: write
   the program that would be wrong if it were accepted.

## Reading

- Lukas Rytz, Martin Odersky and Philipp Haller, *Lightweight Polymorphic
  Effects* (ECOOP 2012): effect polymorphism that's implicit, a function's
  effects relative to its arguments', which an open bound is.
- Daan Leijen, *Koka: Programming with Row Polymorphic Effect Types*
  (2014): every function generic in the effects of what it's given.
