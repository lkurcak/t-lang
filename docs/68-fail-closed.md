# Step 68: abstraction points, failing closed

Code: the mutability check in `src/borrowck/moves.rs`;
`borrowck::drops::may_drop` and `Notes::drops` in `src/mir.rs`; in
`src/check/refine/targs.rs` `changes`, `drop_changes`,
`writable_inside`, `Inst::changed`, `Walk::agree_inv`,
`Walk::check_behind`, `Walk::opaque_result` and `Walk::followed`; in
`src/check/refine/mir.rs` `Walk::bind_ref` and `Walk::mir_drop`; and the
generated family in `tests/generated_soundness.rs`.

Retrospective 7 found three ways to break a refinement that `t check`
accepted, and planning this step found three more, one of them in the
borrow checker. Each is at an *abstraction point*: code that sees a
refined place only as something more general, a type parameter, an
opaque type, an impl's parameter, a pattern's binding, an aggregate's
field, a drop. And each was accepted because a case the checker didn't
handle fell to "nothing's declared there". This step fixes the six, makes
the checker refuse where it can't tell, and tests the products of
abstraction points and positions together, rather than each feature on
its own:

```rust
use alloc::list::List;

trait Go {
    fn go(self);
}

impl<'a> Go for &'a mut usize {
    fn go(self) {
        *self = 99;
    }
}

fn run<G: Go>(g: G) {
    g.go();
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        run(&mut l[0]);
    }
}
// error: can't prove `t < 5` of what `run` may write behind argument `g`: `G: Go` can make or change a value of it
```

## Writes through a shared `&mut`

The borrow checker asked of a place written to, or lent `&mut`, whether
the *last* reference on its path was a `&mut`. So `**a`, of an `a:
&&mut usize`, could be written: but two `&`s to one `&mut` are two
aliases of it, and a `push` through one, while the other holds an
element, leaves that dangling. Now a `&` anywhere on the path refuses, as
in Rust:

```rust,error
use alloc::list::List;

fn main() with alloc {
    let mut l: List<u64> = List::new();
    l.push(7);
    let r = &mut l;
    let a = &r;
    a.push(8);
}
// error: cannot borrow `**a` as mutable: it's behind a `&` reference, `a`
```

## Consumers change what's invariant

Step 60 let a call's type parameter be refined when only *consumers*
bound it, traits whose methods take `Self` and give none back: they
can't make a value of it, so what the callee gives back of it is what it
was given (lemma A5 of docs/60). But an impl of one for a `&mut` writes
through it, and one for a `Cell` sets it, as the example at the top does.
So a consumer that isn't a marker now *changes* the values where the type
it's given is invariant, behind a `&mut` or in an `Unsafe`, and a call
lends a place refined there only if the instantiation keeps nothing
there. Where nothing can write, as in a list of refined numbers given to
code bounded by `Ord`, nothing changes: `max` and `min` still keep their
refinements. The same holds of an impl's consumer bound, for what its
parameter is (`parametric_impl`), and of a projection's values
(`subst_tree`).

## Opaque results

A function returning `impl Iterator<Item = &'a mut usize>` over a refined
list's `iter_mut` gave its caller `&mut usize`s, through which it wrote
what it liked: the result's type was opaque, so the check of what may be
written through a result found no positions in it. Now a result with
opaque types in it is checked as the argument of a function generic in
each, bounded by what its bounds say, which is what the rest of the
program is: an equality bound `Item = &'a mut usize` can make or change
a value there (`makes`), so what's declared there must be nothing.

```rust,error
use alloc::list::List;

fn it<'a>(l: &'a mut List<{ t: usize | t < 5 }>) -> impl Iterator<Item = &'a mut usize> + Discard {
    l.iter_mut()
}

fn main() {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    for r in it(&mut l) {
        *r = 99;
    }
}
// error: can't prove `t < 5` of what may be written through the result of `it`, `impl Iterator<Item = &'a mut usize> + Discard`: `Self::Item = &'a mut usize` can make or change a value of it
```

An opaque result that gives only `&`s keeps everything, as nothing can
write through them; returning the concrete `IterMut<'a, { t: usize | t <
5 }>` says what's declared. Step 68.1 lets an opaque type's bounds say it
too.

## `&` bindings to `Cell`s

A pattern's `&mut` binding to a variant's field, or an array's element,
says what's declared there (step 66). A `&` binding said nothing, which
was right for most types, as nothing writes through a `&`, but not for a
`Cell`, whose `set` takes `&self`:

```rust,error
use core::cell::Cell;

fn main() {
    let o: Option<Cell<{ d: usize | d < 10 }>> = Some(Cell::new(1));
    match &o {
        Some(c) => c.set(99),
        None => {},
    }
}
// error: can't prove `d < 10` for argument `value` of `set`
```

A `&` binding now takes what's declared at its place, as a `&mut` one
does (lemma C1).

## One type, two places

A struct whose two fields are `&mut T` holds two places of one type. A
generic function given it can swap what they point to. If one place is
refined and the other isn't, the swap breaks the first. A call checks
what the callee may write against what's declared of each place it's
lent, but of a value holding two, it took what's declared there as
either's, when it must be both's. Now a struct, a variant or an array
made of places of one type parameter needs them to agree, where the
parameter is invariant, as they must if refinements were types (lemma
A8):

```rust,error
use alloc::list::List;

#[derive(Discard)]
struct W2<'a, T> { a: &'a mut T, b: &'a mut T }

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    let mut y: usize = 99;
    if l.len() > 0 {
        let _w = W2 { a: &mut l[0], b: &mut y };
    }
}
// error: can't prove `t < 5` of what may be written through field `b` of `W2`: it's of the same type as field `a` of `W2`, which has it
```

## Drops are calls

Dropping a value calls its type's `discard`, if it has one of its own,
which may do what a total function may (docs/20). The refinement walk
didn't see drops: an `impl Discard for W<'a, usize>` could write through
its `&mut` field, at the end of the scope of a `W` made of a refined
place. Now each place drop elaboration may drop, at each statement
(`may_drop`, from its own dataflow), is checked as given to that
`discard`. When the `discard` isn't parametric in what's there
(`drop_changes`), nothing may be declared where the value's type is
invariant. A `Discard` bound counts as a consumer where what it's given
has such a `discard`. A `List<T>`'s is parametric in `T`: it drops each
element, by the element's.

```rust,error
use alloc::list::List;

struct W<'a, T> { r: &'a mut T }

impl<'a> Discard for W<'a, usize> {
    fn discard(self) {
        *self.r = 99;
    }
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        let _w = W { r: &mut l[0] };
    }
}
// error: can't prove `t < 5` of what dropping `_w` may write: `W<usize>`'s `discard` can change a value of it
```

## Failing closed

Each hole above was a case the checker didn't handle, which fell to
"nothing's declared there". The cases now say what's declared, and
where the checker can't tell, it refuses:

- `bind_ref`'s match has no catch-all arm: each kind of binding says what
  its reference declares.
- A position that a tree says something at, but that the walk can't
  follow in the type (`followed`), refuses rather than being skipped. No
  program in the tests or downstream reaches one, so this costs nothing.
- A reference to a known place also takes that place's declaration, not
  only what the reference says.
- What the checker can't tell refuses: an opaque type whose hidden type
  isn't known, a `Discard` bound whose type at the call isn't known, a
  type deeper than the walk follows, and a type parameter, opaque type or
  projection as what's inside a type given to a consumer.

## Abstraction points, positions, traits

Every hole of retrospectives 6 and 7 was in a product of two features,
each tested on its own: `Cell` and pattern bindings, consumers and
`&mut` instantiations, opaque results and refined lists. The generated
family in `tests/generated_soundness.rs` builds each product of

- a position: a refined list's element given `&mut`, or a refined `Cell`
  given `&`;
- a trait: a marker whose `discard` writes, a consumer, a maker
  (`fn re(self) -> Self`), an equality bound that gives it back;
- an abstraction point: a generic function, an opaque result, an impl
  generic in it;

and the pattern, aggregate and drop points on their own, each writing a
value that breaks the refinement and one that keeps it. The interpreter
is the oracle: each program is refused, or runs without "type soundness
violated". On 0.81.0, 25 of the 62 broke a refinement, among them
products no hand-written test had, such as a maker through an impl's
bound. Now none do. A second test checks that a value that keeps the
refinement, written through a binding the checker can see, is accepted.
A third checks that every program the checker refuses is refused for the
refinement, not for a mistake in the template.

## Why it's sound

- **Lemma B1 (shared means shared).** What a `&` points to may be read
  by every alias of it at once, so nothing reached through it may change
  but through an `Unsafe`. A `&mut` behind a `&` is reached through it. So
  a place may be written, or lent `&mut`, only if no reference on its
  path, past what a closure's environment is, is a `&`. That's Rust's
  rule, and RustBelt's: a shared reference to a mutable one is a shared
  reference.
- **Lemma A5 (consumers, restated).** A consumer's method gives back no
  value of `Self`, so code generic in `X: C` gives back only values of
  `X` it was given. But it may change what's *in* one where `X`'s type is
  invariant: an impl of `C` for a `&mut U` writes a `U`, one for a
  `Cell<U>` sets one. Where it's covariant, nothing can write, through
  any alias. So a consumer bound keeps the instantiation at covariant
  positions and knows nothing at invariant ones (`Inst::changed`).
  Markers have no methods; `Discard`'s is `discard` (A9).
- **Lemma A8 (one type, two places).** If refinements were types, a
  `W2<T>` holding `&mut`s to two places would have one `T`, which, being
  invariant, would be what each place declares, exactly. So the places
  must agree there, and then what's declared at `T`'s positions is either
  one's: the same. Of the values (covariant), it's either, which is what
  the walk already says.
- **Lemma A9 (drops).** A drop of a value whose type has its own
  `discard` is a call of it, given the value. If that `discard` is
  parametric in what the value holds (its impl is generic in each type
  argument, and its bounds are markers or are on types whose drops are
  such), it writes only values it was given, by lemma A2, of what's
  there, which A8 makes agree. If not, it may write anything at each
  invariant position, so nothing may be declared there. Drop elaboration
  drops exactly the places its dataflow says may hold a value
  (invariant E1 of docs/32.1), and `may_drop` reports those, from the
  same dataflow, for the copy the walk sees: so every drop is checked.
- **Lemma A10 (opaque results).** The rest of the program sees a
  function's opaque result only through its bounds: it's code generic in
  a type with those bounds, given the value. So checking the result as
  that code's argument (A3, A5, W5, W6) checks every way the program can
  write into the hidden value.
- **Lemma C1 (`&` bindings).** A `&` binding to a value at no place, a
  variant's field or an array's element, is a reference to it. Writes
  through it (in a `Cell`) must keep what's declared there, which is
  what it now says, as a `&mut` binding's does (V1 of docs/66).
- **Lemma F1 (references).** What's declared behind a reference is what
  every place it may point to declares. That's what it says where it was
  made (W1 of docs/63), and, when the place it points to is known, what
  that place declares: either is necessary, so both are checked.
- **Lemma F2 (positions).** A tree's entry at a position the type can't
  be followed to would go unchecked. Refusing there keeps every entry
  checked.

## Tested by switching rules off

Each rule of this step was switched off in turn, sixteen times, and the
golden tests and the generated family run each time. Twelve made a test
fail; `consume` needed a test of its own first (a linear value taken
apart by a `match`, whose unbound part is dropped there). Four survive,
each for a reason:

- A consumer bound on an impl's parameter, in what a projection gives
  (`subst_tree`): a call reaches that only through a trait that isn't a
  consumer, and `parametric_impl` already makes such a call know
  nothing, as consumers have no associated types. It stays for what
  lemma A5 says, in case that changes.
- A reference to a known place taking that place's declaration, in two
  places (F1), and refusing a position the walk can't follow (F2): no
  program in the tests reaches a case where they matter, as the
  failing closed section says. They're defence in depth, against a case
  later steps add.

## Decisions

- **Fail closed** (D1 of retrospective 7): where the checker can't tell
  what's declared, it refuses, and the cases that decide are explicit.
- **Consumers change what's invariant** (D2): impl-aware precision, a
  consumer whose impl for the instantiation writes nothing, is a later
  relaxation.
- **Drops are calls**, with `Discard` a consumer only where the
  instantiation's drop can write. No rule against a `Discard` impl for
  one instance (Rust's E0366) was needed.
- **Associated-effect bindings and refined opaque bindings** are step
  68.1, released on their own (0.83.0), so the holes' fixes ship first.
- **0.82.0.** Programs that were accepted are refused: each was unsound,
  or wrote through a shared `&mut`.

## What it doesn't do

- A consumer bound refuses a call lending a refined place where its
  instantiation is invariant, even if no impl of it writes.
- An opaque result can't say what's declared inside it yet: `impl
  Iterator<Item = &'a mut { t: usize | t < 5 }>` is refused (step 68.1).
- A caller of a function returning `impl Iterator` can't name its
  effects in a `with`, so only one that infers its effects can iterate it
  (step 68.1).
- A value taken apart by a pattern is checked as dropped whole, though
  its own `discard` isn't called then: refusing more than needed, for a
  type with a `discard` that isn't parametric, in its own module.
- What's declared behind each reference is still found two ways, as
  lemma F1 says, not kept in one place with a "don't know" of its own.

## Exercises

1. Write the data race lemma B1 rules out: two threads, each with a `&`
   to one `&mut u64`, adding to it. Which bound of `thread::scope`'s let
   it run?
2. `trait Show { fn show(&self) -> u64; }` is a consumer. Which
   instantiations of `fn f<T: Show>(x: &T)` can change what's in `x`?
   Write one.
3. Why must the two places of lemma A8 agree exactly, and not just one
   imply the other? Write the program that breaks if `b`'s declaration
   only implies `a`'s.
4. A `Discard` impl generic in `T` with the bound `T: Discard` is
   parametric. What could it do with a `T: Int` bound instead, and which
   rule of docs/20 already refuses that?

## Reading

- John C. Reynolds, *Types, Abstraction and Parametric Polymorphism*
  (IFIP 1983), and Philip Wadler, *Theorems for free!* (FPCA 1989): what
  code generic in a type can do with its values, which lemmas A5 and A9
  restrict to what its bounds allow.
- Ralf Jung, Jacques-Henri Jourdan, Robbert Krebbers and Derek Dreyer,
  *RustBelt: Securing the Foundations of the Rust Programming Language*
  (POPL 2018): shared references as read-only for what they reach, but
  for interior mutability, as lemma B1 says.
- The Rust reference on `Drop`, and RFC 1238 (*nonparametric dropck*):
  why a destructor's being generic in what it holds matters.
- Nico Lehmann, Adam Geller, Niki Vazou and Ranjit Jhala, *Flux: Liquid
  Types for Rust* (PLDI 2023): refined references, and opaque types'
  refinements.
