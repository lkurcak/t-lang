# Step 77: captures in methods

Code: where a closure's capture stops, in `Checker::cut`
(`src/check/closures.rs`), which says why when it stops short; the
function parameters it may reach through, `Checker::fn_params`
(`src/check.rs`); which fields a refinement relates,
`RefinedFields::related` (`src/check/refine.rs`); the note on a borrow
that clashes, `Loan::why` (`src/borrowck/build.rs`, `clash` in
`src/borrowck/loans.rs`); and the refinement checker's backstop,
`Walk::captures_apart` (`src/check/refine/mir.rs`).

Step 76 made closures capture the places they name (docs/76), but a
closure that changed a place stopped at the first `*`: in a method, one
changing `self.total` captured all of `self`. Retrospective 10 found the
two places that hurt most, refused by 0.91.0:

```text
use alloc::list::List;

#[derive(Discard)]
struct S { items: List<u32>, total: u32 }

impl S {
    fn sum(&mut self) with panic {
        let mut add = |x: u32| self.total = self.total + x;
        // error: cannot borrow `self.items` while `self` is mutably borrowed
        for x in &self.items {
            add(*x);
        }
    }
}

fn main() {}
```

Rust has accepted that since 2021. And a struct with any refined field
was captured whole, even for a field no refinement mentions:

```text
use alloc::list::List;

#[derive(Discard)]
struct P { lo: u32, hi: { h: u32 | h >= lo }, log: List<u32> }

fn main() with alloc {
    let mut p = P { lo: 0, hi: 1, log: List::new() };
    let mut note = |x: u32| p.log.push(x);
    // error: cannot assign to `p.lo` while `p` is mutably borrowed
    p.lo = 1;
    note(3);
}
```

Both are accepted now: `add` captures `(*self).total` by `&mut`, and
`note` captures `p.log`.

```rust
use alloc::list::List;

#[derive(Discard)]
struct S { items: List<u32>, total: u32, log: List<u32> }

impl S {
    fn sum(&mut self) with {panic, alloc} {
        let mut add = |x: u32| self.total = self.total + x;
        let mut note = |x: u32| self.log.push(x);
        for x in &self.items {
            add(*x);
            note(*x);
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut out = os.stdout;
    let mut s = S { items: List::new(), total: 0, log: List::new() };
    s.items.push(2);
    s.items.push(5);
    s.sum();
    out.line(&f"{s.total} {s.log.len()}") // expect: 7 2
}
```

## Where a changing capture stops

K1, K3 and K4 of docs/76 are as they were: by `&`, a closure captures the
place it names, through anything; moved out, not out of what can't be
taken apart here; overlapping places merge. K2 is now:

> **K2. By `&mut`, uniquely or by value: fields of the variable, and by
> `&mut` or uniquely, through a parameter.** A closure changing a place
> captures it through fields, and through the `*` of a function's
> parameter that's a `&mut`, `self` too, to a field of what it points to,
> if the parameter isn't declared `mut` and its type has no refinement.
> It's cut:
>
> - at any other `*`: of a reference in a variable, of a closure's
>   parameter, of a parameter declared `mut`, of a box, or a second one;
>   and by value, every `*`, as nothing's moved out from behind a
>   reference;
> - at a variable declared with a refinement;
> - before a field a refinement relates to another field of its struct:
>   one whose refinement names another, or that another's names;
> - after a field with a refinement of its own: it's captured whole.

A `*r` the body names whole is captured as `r`, uniquely, as before.

So in a struct whose fields' refinements relate some of them, a closure
can change each of the others apart, and a field whose refinement is
about it alone, whose writes keep it:

```rust
#[derive(Discard)]
struct R { lo: u32, hi: { h: u32 | h >= lo }, count: { c: u32 | c < 100 }, log: u32 }

fn f(r: &mut R) with panic {
    let mut note = || r.log = r.log + 1;
    let mut reset = || r.count = 0;
    if r.hi < 1000 {
        r.hi = r.hi + 1;
    }
    note();
    reset();
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut r = R { lo: 0, hi: 1, count: 7, log: 0 };
    f(&mut r);
    out.line(&f"{r.hi} {r.count} {r.log}") // expect: 2 0 1
}
```

Where a capture is cut short of what the body names, and that clashes, a
note says why:

```rust,error
#[derive(Discard)]
struct R { lo: u32, hi: { h: u32 | h >= lo } }

fn f(r: &mut R) {
    // `r` is mutably borrowed here: the closure captures `r`, not just
    // `r.lo`, as a refinement of `R` relates `lo` to `hi`
    let mut c = || r.lo = 5;
    r.hi = 3; // error: cannot assign to `r.hi` while `r` is mutably borrowed
    c();
}

fn main() {
    let mut r = R { lo: 0, hi: 10 };
    f(&mut r);
}
```

## Why it's sound

Lemma C4 of docs/76 is what K2 is for: a refinement mustn't relate a
place in a changing capture to one outside it, as the walk of the
closure's body would check what the closure writes against the outside
as it was when the closure was made, which the function may have changed
since. Three lemmas say the new K2 keeps that.

> **Lemma E1 (behind a parameter is its type).** Nothing assigns a
> function's parameter that isn't declared `mut`, so what the refinement
> checker has declared behind it, if it's a `&mut`, is what its written
> type says: never what a `let` or a `&mut` of a refined variable would
> make it say (docs/63-what-is-declared.md). With no refinement written
> in its type, nothing is declared of what it points to, as a whole or at
> its type arguments' positions, and what's about its fields is the
> struct's field refinements, which E2 and E3 deal with. A refinement the
> caller has of what it lent isn't the function's to keep: the caller
> proves it again after the call (docs/74). A `mut` parameter could be
> made to point to what another parameter's type refines; a reference in
> a variable to a refined variable; a closure's parameter to what a
> refined type argument says where the closure's called: those are cut.

> **Lemma E2 (apart fields).** A field no refinement relates to another
> field has, at most, a refinement of its own naming nothing else. A
> write to it keeps that, checked against the value written, never
> against the rest. A refinement elsewhere naming the field, a `let`'s,
> is of the value it had then: only a `mut` variable's is checked again,
> and that can't name a field behind a `&mut` (docs/56, *What can
> change*); so it's a fact about a value, as a `&` capture's are
> (lemma C5).

> **Lemma E3 (a refined field is whole).** A field's own refinement can
> relate the places in it: `q: { q: Q | q.a < q.b }`, or a type
> argument's refinement in its type, about each element of a list.
> Captured whole, everything it relates is in the capture.

So C4 holds of the new K2, and so do C1 to C3: a capture through a
parameter is the borrow `&mut (*r).x`, whose loan the borrow checker
treats as it does any reborrow, and which the refinement checker forgets
as it does any loan (docs/69-loans.md); every capture is a place in
what was captured before, so nothing accepted is refused. Being of
`(*r).x` rather than of `r`, it can live as long as `*r` is lent, not
just while `r` is in scope, as in Rust.

The refinement checker's backstop now checks K2 as it is
(`Walk::captures_apart`): no refinement on the variable; only fields,
each apart, and a refined one last; a `*` only first, of a `&mut`, by
`&mut`, to a field, where nothing's declared behind it and the variable
isn't a reference to a place. Otherwise, an internal error.

The adversarial tests are `tests/t/err_soundness_capture_*.t` and
`err_capture_through_box_write.t`, and the generated family in
`tests/generated_captures.rs`, now with the root a variable, a `&mut`
parameter, `self`, a parameter declared `mut`, a reference in a variable
or a box, and structs whose refinements relate nothing, the field to its
sibling, the field to itself alone, the places in its parent, or its
parent to another field: 960 programs, each refused where what the
function does clashes with what the closure holds, by the rules, and
each other run.

## What it doesn't do

- **Through boxes.** A closure changing `b.x` captures `b`. A box's
  value is the box's, and what's declared of it changes when it's
  written (docs/60); going through would need the argument E1 makes for
  parameters, for boxes too.
- **Through references in variables.** `let r = &mut p;` then `|| r.x =
  1` captures `r`: the reference can say what's declared of `p`.
- **Fields of a refinement's component.** A closure changing both `lo`
  and `hi`, which a refinement relates, captures the struct, not the two
  fields, though nothing outside them is related: a capture is one place.
- **By value, relations.** A `move` closure copying a field a refinement
  relates to another is cut at the struct like a changing one, though
  what it changes is its own copy.

## Decisions

- **D1: through a parameter's reference.** A function's parameter not
  declared `mut`, without refinements, to a field; not Rust's whole rule,
  every reference, with the refinement checker refusing where something's
  declared behind it; nor through boxes.
- **D2: fields apart, both ways.** A field no refinement relates to
  another is captured alone, by `&mut` or by value; a refined field
  whole.
- **D3: a note.** A borrow error from a capture cut short of what the
  body names says why.

## Exercises

1. Write the program a `mut` parameter would break if a closure captured
   `r.x` through it. Does the refinement checker refuse it some other
   way? (What does the function know of `r.x` while it's lent?)
2. Why is a `*r` the body names whole captured as `r`, uniquely, rather
   than `*r` by `&mut`? What would change for a closure that calls
   `self.put(e)`?
3. E3 captures `p.q` whole when `q: { q: Q | q.a < q.b }`. When could a
   closure changing `p.q.a` capture only that, and what would the
   refinement checker need to know?
4. The last item of *What it doesn't do*: why is cutting a `move`
   closure's copy at a refinement never needed for soundness? What does
   the walk of its body check its writes against?

## Reading

- RFC 2229, *Closures capture disjoint fields*, on truncating at derefs:
  Rust goes through every reference for a borrow, and through none for
  a move closure.
- Niko Matsakis, *Closures capture disjoint fields*, the design notes for
  RFC 2229.
