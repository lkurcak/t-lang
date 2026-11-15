# Step 79: what a capture splits

Code: where a closure's capture stops, `Checker::cut` and
`Checker::at_type_argument` (`src/check/closures.rs`), with the closures'
parameters it stops at, `Checker::lambda_params` (`src/check.rs`); the
refinement checker's backstop, `Walk::captures_apart`
(`src/check/refine/mir.rs`), which now asks the walk what's declared of
each place a capture is in (`Walk::place_decl_own`,
`src/check/refine/targs.rs`); and how a message says a condition,
`Disj::condition`.

Retrospective 11 found a hole that came with 0.91.0 (step 76, closures
capture the places they name): a closure changing a place inside a
tuple's element, or inside a generic struct's field, could break a
refinement the element's type carries. 0.91.0 to 0.93.0 accepted this,
and `need` read a `t.0` whose `x` was more than its `y`:

```rust,error
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn make() -> ({ v: Q | v.x <= v.y }, u32) { (Q { x: 0, y: 10 }, 0) }

fn need(q: &{ v: Q | v.x <= v.y }) -> u32 { q.y - q.x }

fn main() {
    let mut t = make();
    let mut c = || t.0.x = 50; // error: can't prove `v.x <= v.y` for `t.0`, once `t.0.x` is written
    c();
    let _ = need(&t.0);
}
```

`t` is declared with no refinement, but its type, taken from `make`'s,
says one of `t.0`: a `let` without a refinement declares what its
value's type has (docs/63-what-is-declared.md), and every read of `t.0`
assumes it. Step 76 cut a changing capture at the refinements the checker
can see, a variable's and a struct's fields'. The type a `let` takes from
its value only the refinement checker knows, after. So `c` captured
`t.0.x` alone, by `&mut`; the closure's write went through that
reference, which says nothing of `t.0`; and nothing checked `t.0`'s
refinement. Every other way to change `t.0.x` was refused: `&mut t.0.x`
can't be made, as `t.0`'s type has a refinement. 0.90.0 captured `t`
whole, and refused the program as it does now.

## Where a changing capture stops

K1, K3 and K4 of docs/76 are as they were. K2 (docs/77) stops at two more
places:

> **K2a. Not below a type argument's place.** A capture by `&mut`,
> uniquely or by value that goes below a tuple's element, or below a
> struct's field whose type is declared as a type parameter (or a
> projection, `T::Out`), stops there: the element or the field is
> captured whole. Whether that type argument has a refinement is the
> refinement checker's to know, so this goes by the type as declared:
> `p.a.x` of a `Pair<T>`'s field `a: T` captures `p.a`, whatever `T` is.
>
> **K2b. Not into a closure's parameter.** A closure changing a place in
> a closure's parameter (one closure in another) captures the parameter:
> what's declared of it is what the call the outer closure is given to
> gives it (docs/60; docs/63, *Closures given*), which only the
> refinement checker knows.

So the capture is the whole the refinement is of, and what the closure
writes is checked against it, as an assignment outside a closure is. A
write that keeps the refinement, whatever the rest of the whole is (the
capture's loan forgets it, docs/69), is accepted, and what's next to the
whole is still apart:

```rust
#[derive(Discard)]
struct Q { x: u32, y: u32 }

#[derive(Discard)]
struct Pair<T> { a: T, n: u32 }

fn make() -> ({ v: Q | v.x <= v.y }, u32) { (Q { x: 3, y: 10 }, 0) }

fn pair() -> Pair<{ v: Q | v.x <= v.y }> { Pair { a: Q { x: 3, y: 10 }, n: 0 } }

fn need(q: &{ v: Q | v.x <= v.y }) -> u32 { q.y - q.x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut t = make();
    // Captures `t.0`: what it writes keeps `t.0`'s refinement.
    let mut c = || t.0.x = 0;
    t.1 = 7;
    c();
    let mut p = pair();
    let mut d = || p.a.y = 4294967295;
    p.n = 1;
    d();
    out.line(&f"{need(&t.0)} {t.1} {need(&p.a)} {p.n}") // expect: 10 7 4294967292 1
}
```

Where that clashes, the note says why:

```rust,error
#[derive(Discard)]
struct Q { x: u32, y: u32 }

fn make() -> ({ v: Q | v.x <= v.y }, u32) { (Q { x: 0, y: 10 }, 0) }

fn main() {
    let mut t = make();
    // `t.0` is mutably borrowed here: the closure captures `t.0`, not
    // just `t.0.x`, as the type of `t.0` is a type argument, which may
    // have a refinement relating the places in it
    let mut c = || t.0.x = 5;
    t.0.y = 3; // error: cannot assign to `t.0.y` while `t.0` is mutably borrowed
    c();
}
```

## Where what's declared comes from

The refinement checker keeps, for each place, what's *declared* of it:
what every write there must keep, and every read may assume
(`Walk::place_decl`, lemma A6 of docs/60). A changing capture must not
stop below a place with something declared of it as a whole, and the
checker's cut runs before the refinement checker, so it has to know
every way something comes to be declared. There are six:

1. **A refinement written on a variable**, a `let`'s or a parameter's
   (`Walk::declare`): an invariant of the variable, and what it carries
   at its type arguments' places. K2 captures the variable whole.
2. **A variable without one**, a `let`'s, or what a `for`, a `match` or a
   `let` taking a value apart binds by value (`targs::widen`): what its
   value's type has. Only a type has a refinement there (a value's own
   facts aren't declared), and only at a type argument's place, never of
   the whole variable. K2a stops at those places.
3. **What a pattern binds by reference** (`Walk::bind_ref`): the same,
   of a reference. A capture stops at its `*`.
4. **A closure's parameter**: what the call the closure is given to
   gives it (`targs::closure_trees`), of the whole of it too. K2b.
5. **A field**: what's declared of the struct at the field's type
   arguments' places (2 again: K2a, a field at a time), and the field's
   own refinement, whole (E3 of docs/77) or of what it relates (E2).
6. **What a reference or a box points to**: what the reference says. A
   capture stops at every `*` but the first of a function's parameter
   with no refinement written in its type, behind which nothing is
   declared (E1 of docs/77).

## Why it's sound

> **Lemma O (origins).** What's declared of a place comes from one of
> the six above, and K2, K2a and K2b stop a changing capture at each, so
> nothing is declared of the whole of any place a changing capture is
> strictly in. By cases: (1) the variable is captured whole; (2) a
> widened tree has `Pred`s only, from types, and a type's refinements
> are at its type arguments' places, which a path from the variable
> reaches only through a tuple's element or a field declared as a type
> parameter or a projection (through a field of another type, the next
> struct's fields are reached the same way), where K2a stops; (3) and (6)
> are behind a `*`, where K2 stops, but for E1's; (4) is the closure's
> parameter, where K2b stops; (5) is lemmas E2 and E3, and K2a.

> **Lemma C4 (refinements don't straddle a capture), corrected.** The
> walk of a closure's body checks a write to a place in a capture
> against what's declared of every place it's in, up to the capture
> (`Walk::keep_wholes` stops at the capture's reference). By lemma O,
> nothing declared of a place a changing capture is strictly in is left
> unchecked, and a struct's field refinements relating a place in it to
> one outside are cut by E2. So what the closure changes is checked
> against all it must keep, of values in the capture.

The step of docs/76's C4 that failed said "refinements of type arguments
are of values, not places (lemma A1 of docs/60)". A1 says a type
argument's refinement names nothing outside the value at its place: it
is closed. It still relates the places *inside* that value, `v.x <=
v.y` of `t.0`, as a field's own refinement does (E3 of docs/77). So a
capture splitting `t.0` was as wrong as one splitting a refined field,
which E3 already cut.

> **Lemma B (the backstop fails closed).** After the cut, the refinement
> checker refuses, as an internal error, a changing capture strictly
> inside any place that has something declared of it as a whole, by
> asking the walk itself at every place the capture is in
> (`Walk::place_decl_own`), and not the rules' list. A way of declaring
> something lemma O missed is so refused, not accepted.

The backstop of 0.91.0 to 0.93.0 checked the capture against the same
list the cut used (a variable's refinement, a struct's field
refinements), so it couldn't catch what the list left out. Switching
K2a and K2b off, every program of the hole is refused by the backstop.

## Saying what can't be proved

Where what's declared at a place says which values may be there, rather
than a predicate (a closure's parameter's, the values the call has of
its type), or that none may (a type parameter the call has no value of),
a message said "can't prove \`\`". It says so in words now:

```rust,error
fn h<R: Discard, G: FnOnce(&mut Option<R>)>(g: G) -> Option<R> {
    let mut s = None;
    g(&mut s);
    s
}

fn main() {
    let r = h(|s| *s = Some(99usize)); // error: can't prove `false` for what's written through `*s`
    let _ = r;
}
```

`h` has no value of `R` of its own, so what a closure it's given writes
behind the `&mut` must be `None` (docs/74). Rust accepts this; T may
too, when a closure writing a value of `R` counts as a source of them.

The adversarial tests are `tests/t/err_soundness_capture_*.t`
(`tuple_element`, `generic_field`, `carried_loop`, `carried_autoref`,
`bindings`, `closure_param_field`, `closure_param_tuple`), and the
generated family in `tests/generated_origins.rs`: each origin above
(with a refined variable, parameter, `&mut` parameter, struct field,
related fields, tuple element, generic field, nested generic field,
closure parameter, `for`, `match`, destructuring and copy) × each way to
change a place in it (assigned, lent to a function or a closure,
through a reference in a variable, a pattern's `&mut` binding, a method
on it, `swap`, and in a closure by `&mut`, by a method, or moved),
writing a value that breaks the refinement or keeps it: every one
breaking it is refused, none accepted breaks a type at run time.

## What it doesn't do

- **Type arguments with no refinement.** `p.a.x` of a `Pair<Q>` is
  captured as `p.a` too: the checker can't tell `Q` from a refined one
  before the refinement checker runs.
- **By value.** `move || t.0.x` takes `t.0`, not a copy of `t.0.x`, as
  0.90.0 took `t`. A copy can't break what's declared of `t.0` (docs/77,
  exercise 4), but K2 cuts all changing captures alike.
- **Changing a closure's parameter.** What's declared of `t` in `|mut
  t| ..` is which values the call has of its type, so `t.x = 5` in the
  closure is refused unless the value written is one of them, whether a
  closure in it writes it or not.

## Decisions

- **D1: capture the whole.** Of three ways to close the hole: (a) cut a
  capture at the place a type argument's refinement could be about, as
  E3 does for a refined field; (b) keep the cut and have the refinement
  checker refuse a capture inside a refined whole, as it refuses
  `&mut t.0.x`, which refuses writes that keep the refinement too; (c)
  check what the closure writes against the whole, with the rest
  forgotten, the most precise and the most work. (a), with a backstop
  that asks the walk.
- **D2: the origins, listed and tested.** The six ways something is
  declared, as lemma O; the generated family over them × the ways to
  change a place.
- **D3: the message says it in words.** `false`, or which values,
  rather than an empty predicate.

## Exercises

1. Why does a `let` without a refinement never have anything declared of
   its whole value (origin 2), when its value came from a call whose
   result type is refined? What would go wrong if it did?
2. Write the program K2b is for, and say why K2a doesn't catch it.
3. K2a cuts `p.a.x` of a `Pair<Q>` too. What would the checker need to
   know, before the refinement checker runs, to cut only where a
   refinement is?
4. The old backstop re-checked the rules' own list. Give another rule in
   T whose backstop could have the same blind spot, and say what it
   would ask instead.
5. Under option (b) of D1, which program in this document's examples
   would be refused?

## Reading

- Jerome Saltzer and Michael Schroeder, *The Protection of Information
  in Computer Systems* (1975): the principle of fail-safe defaults,
  "base access decisions on permission rather than exclusion", which a
  cut that splits unless a listed reason stops it breaks.
- RFC 2229, *Closures capture disjoint fields*, and the Rust reference's
  *Closure expressions*: Rust truncates a capture where its type makes a
  precise one unsound, at a union's field and in a packed struct; T's
  K2 truncates where a refinement could make one so.
