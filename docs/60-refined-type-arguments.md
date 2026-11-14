# Step 60: refinements in any type argument

Code: `src/check/refine/targs.rs` (`Walk::call_trees`,
`Walk::instantiate_one`, `Walk::values_at`, `Walk::assume_tree`,
`Walk::place_decl`, `consumer`, `makes`, `reach`); `Checker::refined_arg`
and the walk's hooks in `src/check/refine.rs`; `Interp::reach` in
`src/interp.rs`; `max` and `min` in `core/cmp.t`.

Step 53 let a refinement be a type argument of an enum, as
`Option<{ d: u8 | d < 10 }>`: the logic knows what an enum's value is made
of, so it can say so of each variant's fields. What's in a list it doesn't
know: so a graph whose every edge was proved to go to one of its nodes,
when the edge was added, forgot it as soon as the edge was in a `List`, and
each use checked it again. Now a refinement can be any type argument:

```rust
use alloc::list::List;
use core::num::wrapping_add;

// The numbers below `n`.
fn below(n: usize) -> List<{ t: usize | t < n }> with alloc {
    let mut l: List<{ t: usize | t < n }> = List::new();
    for i in 0..n {
        l.push(i);
    }
    l
}

// Each element indexes the array without a check.
fn total(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> u64 {
    let mut s: u64 = 0;
    for i in l.iter() {
        s = wrapping_add(s, a[*i]);
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    out.line(&total(a, &below(4)))?; // expect: 1111
    Ok(())
}
```

`List<{ t: usize | t < n }>` is a list each of whose elements is below
`n`. So is a tuple's element, `(u64, { v: usize | v < n })`, what a
reference or a box points to, `&{ v: usize | v < 4 }`, and a slice's or an
array's elements, `[{ v: usize | v < 4 }; 2]`, at any depth.

## What generic code can't do

`List<T>`'s methods are generic in `T`. They can't look at a `T`, add to
one, or make one: all they can do with a `T` is keep it, move it and give
it back (step 55's free theorems, lemma P1). So every element of a list is
one that was pushed, and if each that was pushed is below `n`, each that
comes out is too. That's what the type says, and each call keeps it:

- what's given where the callee takes a `T`, `push`'s `v`, must be below
  `n`;
- what it gives where it gives a `T`, `pop`'s `Option<T>`, `index`'s `&T`,
  the iterator's `Option<&T>`, is below `n`;
- and what it leaves behind a `&mut` it was lent, the list after `push`, is
  still a list of those.

```rust,error
use alloc::list::List;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    q.push(4);
}
// error: can't prove `t < 4` for argument `v` of `push`
```

Each call works out what its type parameter is, its *instantiation*: from
the arguments, whatever they have where the parameter's type has the
parameter. `l.index(i)` is `List::index(&l, i)`, and `&List<T>` has `T`
where `l`'s type has `{ t: usize | t < 4 }`: so that's what `T` is, and
what it gives is a `&{ t: usize | t < 4 }`. `List::new()` has no argument
with a `T` in it: it can't give a `T` at all, so what it gives is a list
of nothing, which is a list of numbers below anything.

## Through `&mut`, exactly

A list behind a `&mut` can be written through it: so it can only be given
where the parameter's type says exactly what the list's does, as Rust's
`&mut T` is *invariant* in `T`. A list of numbers below 4 isn't a list of
numbers below 5 to a callee that could push a 4 through it:

```rust,error
use alloc::list::List;

fn f(l: &mut List<{ t: usize | t < 5 }>) with alloc { l.push(4) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    f(&mut q);
}
// error: can't prove `t < 4` of what `f` may write behind argument `l`
```

Where nothing can write, a value is weakened as it's given: each list of
numbers below 3 is a list of numbers below 4. How a type varies with each
of its parameters is what the borrow checker works out for lifetimes
(`borrowck::regions::AdtInfo`): through a `&` or a box, owned, it's
*covariant*; through a `&mut`, or in an `Unsafe`, so a `Cell`, it's
invariant. What a `Cell` holds can change through a `&` to it:

```rust,error
use core::cell::Cell;

fn poke(c: &Cell<u8>) { c.set(9) }

fn main() {
    let c: Cell<{ t: u8 | t < 3 }> = Cell::new(1);
    poke(&c);
}
// error: can't prove `t < 3` of what `poke` may write behind argument `c`
```

What's declared of a place is what each write there must keep: of a
variable, its refinements; of one without any, the refinements its
value's type has (not the values it has now: `let mut l = List::new();`
can be pushed anything to, as TypeScript widens a literal's type for a
`let`); of a field, the struct's; through a reference, what the reference
says, which is what the place it was made to had declared. So a `&mut` to
a refined list can be kept in a variable, and what's pushed through it is
checked:

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let r = &mut q;
    r.push(3);
    out.line(&q.len())?; // expect: 1
    Ok(())
}
```

A value that holds a reference can't be given where its type doesn't say
the refinements what may be written through it must keep: not to a
struct's field whose type is a `&Cell<u8>`, not as a function's result of
type `&mut List<usize>`, and not to a call through a function value or a
closure, which has no type parameter to keep them.

## Bounds that make nothing

`T: Ord` lets generic code compare `T`s. It can't make one with that: `cmp`
takes two `&T`s and gives an `Ordering`. A trait is a *consumer* if each
of its methods takes `Self` only as `self`, `&self` or a `&Self`, and gives
back nothing with `Self` in it; if it has no associated types; and if its
supertraits are consumers too. `Eq`, `Ord`, `Debug` and `Display` are; so
is a program's own trait of that shape. The markers, `Copy`, `Discard`,
`Send` and `Sync`, are. A type parameter bounded only by those can be a
refined type, so a heap of `(cost, node)` entries keeps what's known of
each node:

```rust
use core::cmp::max;

fn larger(a: (u64, { v: usize | v < 4 }), b: (u64, { v: usize | v < 4 })) -> { v: usize | v < 4 } {
    max(a, b).1
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b: [u64; 4] = [1, 2, 3, 4];
    out.line(&b[larger((1, 2), (5, 0))])?; // expect: 1
    Ok(())
}
```

`Ord` had `max` and `min` methods, which an impl could write a body of its
own for, making a value: they're `core::cmp::max` and `core::cmp::min`
now, functions, which give back one of what they're given. Step 55's free
theorems count consumer bounds as markers too.

A bound that can make or change a value means the call can't keep a
refinement: `Int`, whose literals are values; `Atom`, which `Volatile::at`
reads from an address; a trait with a `&mut self` method, or one that gives
back a `Self`; and a closure that gives back a value of the parameter.

```rust,error
use alloc::list::List;

trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 9 }
}

fn fill<T: Make>(l: &mut List<T>) with alloc { l.push(T::make()) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    fill(&mut q);
}
// error: `T: Make` can make or change a value of it
```

But `T: Iterator`, where `T` is an `Iter<'_, U>`, is fine: the impl of
`Iterator` for `Iter<'a, U>` is generic in `U`, so it can't make or
change a `U` either, and what `<T as Iterator>::Item` is, `&'a U`, says
what it gives. An impl for one type, `impl Iterator for Wrap<usize>`, can
do as it likes with what's in a `Wrap<usize>`.

## Fields, of the struct they're in

A field's type argument can be refined by the fields before it and by the
field itself: a graph whose every edge goes to one of its nodes says so of
its own list of nodes. Each time what's named changes, the refinement is
proved again: after `push`, what was below the old length is below the new
one.

```rust
use alloc::list::List;

#[derive(Discard)]
struct Node<I> { out: List<I> }

#[derive(Discard)]
struct Graph { nodes: List<Node<{ t: usize | t < nodes.len() }>> }

impl Graph {
    #[measure]
    fn count(&self) -> usize { self.nodes.len() }

    fn add_node(&mut self) -> { id: usize | id < self.count() }
    with alloc
    ensures
        self.count() == old(self.count()) + 1,
    {
        let id = self.nodes.len();
        self.nodes.push(Node { out: List::new() });
        id
    }

    fn add_edge(
        &mut self,
        from: { v: usize | v < self.count() },
        to: { v: usize | v < self.count() },
    ) with alloc ensures self.count() == old(self.count()) {
        self.nodes[from].out.push(to);
    }

    // The first edge out of `v` goes to a node: no check.
    fn first_edge(
        &self,
        v: { v: usize | v < self.count() },
    ) -> Option<{ t: usize | t < self.count() }> {
        match self.nodes[v].out.first() {
            Some(t) => Some(*t),
            None => None,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut g = Graph { nodes: List::new() };
    let a = g.add_node();
    let b = g.add_node();
    g.add_edge(a, b);
    out.line(&g.first_edge(a).unwrap_or(9))?; // expect: 1
    Ok(())
}
```

Taking a node away could leave an edge to it, so it's an error:
`self.nodes.pop()` can't prove `t < nodes.len()` of the list after.

## Closures

A closure given to a generic function can count on what the call's other
arguments say of the values of a type parameter it's given, if no closure
gives one back and nothing else can make one: the function can only pass
it those. And only if the closure's type is in no other of the function's
parameters and not in its result, which could give the closure back to be
called with anything later:

```rust
use alloc::list::List;
use core::num::wrapping_add;

fn sum(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> u64 {
    l.iter().fold(0, |s, i| wrapping_add(s, a[*i]))
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    let mut l: List<{ t: usize | t < 4 }> = List::new();
    l.push(3);
    l.push(0);
    out.line(&sum(a, &l))?; // expect: 1001
    Ok(())
}
```

A closure literal is walked after the call's other arguments, as making
one runs nothing.

## Inside generic code

A call of a trait's method was known to be of an impl (lemma R1 of docs/53)
only if its `Self` type had no type parameter in it. Now one inside a type
that's not one itself, `Iter<'_, (usize, E)>` in an `impl<E>`, counts: the
impl `select` finds for it with `E` as it is, is the impl for every type `E`
can be, as any other would overlap it. So a `for` loop inside generic code
gets what the list holds too.

## A hole this step closed

A write through a box changes the value the box is in, as a write to a
field does. The walk treated a box as a reference, which only points to
what changes, so what a measure of the struct a box was in had been was
kept after a write through the box. Every release before 0.76.0 accepted
this:

```rust,error
mod m {
    use alloc::boxed::Box;

    #[derive(Discard)]
    pub struct Inner { pub x: u8 }

    #[derive(Discard)]
    pub struct S { pub b: Box<Inner> }

    impl S {
        #[measure]
        pub fn v(&self) -> u8 { self.b.x }

        pub fn new() -> S with alloc { S { b: Box::new(Inner { x: 1 }) } }
    }
}

fn main() with alloc {
    let mut s = m::S::new();
    let a: [u8; 2] = [7, 8];
    if s.v() == 1 {
        s.b.x = 5;
        let k: { k: u8 | k == 1 } = s.v();
        let _ = a[k as usize];
    }
}
// error: can't prove `k == 1` for `k`
```

Now a write through a box gives the box, and what it's in, new values.
`Box::new(x)` is known to hold `x`, so some programs need `panic` less.

## Why it's sound

- **Lemma A1 (closed predicates).** A refinement in a type argument names
  what can't change: immutable variables, parameters, consts, and in a
  field's, the struct's fields. Where it's said, each name is bound to the
  logic's value it has there (`targs::Closed`); objects never change, so
  it's a fixed predicate of values.
- **Lemma A2 (unary parametricity).** Reynolds' abstraction theorem, for a
  predicate rather than a relation (a *logical predicate*): a function
  generic in `X`, whose bounds respect every predicate on `X`'s values,
  takes arguments each of whose values of `X` has `P` to a result each of
  whose values of `X` has `P`, and writes only such values where it writes
  values of `X`. Markers respect every predicate (a copy is the same
  value); consumers do (lemma A5); so does an impl generic in all of its
  type's type arguments whose own bounds are such. Intrinsics generic in
  `X` move, copy and store the values they're given (lemma P1 of docs/55);
  unsafe code that breaks this breaks its own obligations already.
- **Lemma A3 (instantiation).** A call's instantiation of `X` is, at each
  position of the type `X` is there, the cases of each argument's values
  there: the value itself where the parameter's type is `X`, else what's
  known of the values there. So each argument has its parameter's type with
  `X` that, and by A2 so do the result and what's behind each `&mut` it was
  lent. Where the parameter is invariant, the argument's cases are what's
  declared of the place lent, and the instantiation must have what's
  declared there: so what the callee writes keeps it.
- **Lemma A4 (variance).** Where a type is covariant in a position, a value
  whose values there have `P` has `Q` there if `P` implies `Q`: nothing
  can write there through an alias that thinks `P` (borrowck's variances,
  the same as for lifetimes: lemma U2 of docs/46). Where it's invariant,
  the two must be the same both ways. A value can only get where its type
  doesn't say what's declared of where it can write if a check there says
  it may: at a call, a `let`, an assignment, a field of a literal, a
  variant's field, a result.
- **Lemma A5 (consumers).** A consumer trait's method gives back nothing
  with `Self` in it, and takes `Self` only by value or `&`: what's behind a
  `&` changes only in an `Unsafe`, which no predicate can see (measures
  read no `Unsafe`'s value, and the walk treats one as opaque). *Wrong as
  stated, fixed in step 68*: `Self` may be a `&mut`, which an impl writes
  through, and a `Cell`'s refinement is seen in its `Unsafe` since step 60
  carried it. A consumer makes no value of `Self`, but changes what's
  where its instantiation is invariant (lemma A5 of
  docs/68-fail-closed.md).
- **Lemma A6 (declared trees).** What's declared of a place holds of its
  value whenever it can be read: each write is checked (an assignment
  against what's declared, a lend against the instantiation by A3, a write
  through a reference against what it says), a field's refinement is
  proved again when what it names changes, and a value read anew is one of
  what's declared.
- **Lemma A7 (impls in generic code).** `select` treats a type parameter
  as a type of its own: the impl it finds for a type with one inside
  matches it for every type it can be, and any other impl that matched one
  of those would unify with it, which the overlap check, and the orphan
  rule across packages, rule out. Monomorphisation picks the same one.
- **Closures.** A closure whose type is only the one parameter's can only
  be called by the function it's given to, and code that function gives it
  to, all generic in `X`: by A2, only with values that have `X`'s
  instantiation.
- **The interpreter checks.** `t run --interp` checks each carried
  refinement wherever it checks one of the whole value (a parameter, a
  result, a `let`, a field, an assignment), and a refined variable's each
  time it's used, as a call it's lent to can change what's in it: of every
  value at its position, which it finds through the fields, variants,
  references, boxes, arrays, slices and buffers the checker says lead
  there (`Measures::reach`).

The adversarial tests are `tests/t/err_soundness_targ_*.t` and
`err_soundness_box_write_measure.t`.

## What it doesn't do

- `map`'s and `filter`'s closures can't count on what the iterator holds:
  those adapters give the closure back, inside them, and the checker
  doesn't look at whether anything can get it out again. `fold`'s can.
- What's declared of a place lent `&mut` was only of its type arguments,
  not the place's own refinement: `swap` of two `{ v: u32 | v > 0 }`s
  forgot it. Step 74 adds it (docs/74-refined-instantiation.md).
- A `let mut` without a refinement doesn't declare what's only known of
  its value now: `let mut l = List::new(); l.push(1);` is a list of any
  numbers, not of 1s.
- Nothing is inferred across a loop for a variable without a refinement:
  what's pushed in the loop is what's declared.
- A `&mut` in a struct field whose type doesn't say what's declared of
  what it points to can't be made: the struct would need a type parameter
  for it.

## Decisions

- D1, what a call's type parameter is: what its arguments have, by
  parametricity, rather than only what's written; D3, with borrowck's
  variances, rather than everything exact.
- D2, which bounds: markers and consumers; `Ord`'s `max` and `min` moved
  out to functions; free theorems count consumers too.
- D4, a field's type argument's refinement can name the fields before it
  and the field itself, proved again when they change.
- D5, a tuple's elements can be refined.
- D6, closures count on the call's other arguments, as above.
- D7, the interpreter checks every value at each position.
- D8, a trait's method call inside generic code is known to be of the impl
  `select` finds when the type it's on isn't a type parameter itself.
- And: a `&mut` to a refined place can be kept in a variable.

## Exercises

1. Why must the instantiation, where a parameter is behind a `&mut`, have
   exactly what's declared of the place lent, and not just imply it? Write
   the program that would be wrongly accepted if it only implied it.
2. `fn swap<T>(a: &mut T, b: &mut T)`: what's its instantiation, given
   `&mut q` and `&mut r` of two lists with different refinements? Which
   check fails?
3. Is `trait Size { fn size(&self) -> usize; }` a consumer? `trait Pick {
   fn pick(&self, other: &Self) -> &Self; }`? Write the program the second
   would let through if it were one.
4. Why can a closure given to `fold` count on what the iterator holds, but
   not one given to `map`? What would it take to let `map`'s?
5. A `let mut` without a refinement declares what refinements its value's
   type has, but not what's known of its values now. What would go wrong
   if it declared both?

## Reading

- Vazou, Rondon and Jhala, *Abstract Refinement Types* (2013): refinements
  that are parameters of types and functions.
- Rondon, Kawaguchi and Jhala, *Liquid Types* (2008): refinements of type
  arguments, inferred by instantiation.
- Lehmann, Geller, Vazou and Jhala, *Flux: Liquid Types for Rust* (2023):
  refinements of Rust's type arguments, with ownership doing what
  invariance does here.
- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983), and
  Wadler, *Theorems for free!* (1989).
- The Rustonomicon, *Subtyping and Variance*: why `&mut T` and `Cell<T>`
  are invariant in `T`.
