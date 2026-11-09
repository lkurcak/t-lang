# Step 59: lends past the call

Code: `lend_framed`, `writes`, `Checker::measure_free_in`,
`Checker::pred_mutable` and `Walk::post` in `src/check/refine.rs`;
`is_free` in `src/check/refine/free.rs`; `written_receiver` and
`index_mut_instead` in `src/check/expr.rs`; variant measures in
`Checker::measure_term`, `Walk::measure_term` and `Interp::measure_term`;
`List`'s `index_mut`, `get_mut`, `as_mut_slice`, `slice_mut`, `iter_mut`,
`pop` and `swap` in `alloc/list.t`; `swap` in `core/slice.t`; `is_some`,
`is_none`, `is_ok` and `is_err` in `core/option.t` and `core/result.t`.

`l[i] = v` is `*l.index_mut(i) = v`, and `index_mut`'s result borrows the
list. Until now, a call whose borrow outlived it made the checker forget
everything about what it was lent, so writing into a list forgot how long
it was:

```text
let mut l: List<u8> = List::filled(3, 0);
l[0] = 3;
l[1] = 4;
// error: can't prove `i < self.len()` for argument `i` of `index_mut`
// note: mutable lending invalidated facts about the previous value of `l`
```

That was step 30's "deliberately missing": a function can't prove what's
true when a borrow it hands out ends, as the caller decides when, and
does what it likes until then. But what it can do is limited. `index_mut`
gives a `&mut T`, and `T` is a type parameter: all the caller can write
through it is a `T`, into a place that holds one. A list's length isn't
a `T`. So now `index_mut` says the length stays, and it does:

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<u64> = List::filled(3, 0);
    l[0] = 3;
    l[1] = 4;
    l[2] += l[1];
    let r = &mut l[0];
    *r += 1;
    for x in l.iter_mut() {
        *x *= 10;
    }
    for i in 0..l.len() {
        l[i] = l[i] + 1;
    }
    out.line(&l[0])?; // expect: 41
    out.line(&l[2])?; // expect: 41
    Ok(())
}
```

## What a result can write

A `&mut` parameter is *lend-framed* when the borrow it's lent may outlive
the call only through the result, and the result can write nothing but
values of the function's type parameters. Its type is `&'a mut U`, and
`'a` is in no other parameter, not in `U` and in no bound; the result
isn't an `impl` type; and wherever the result holds `'a`, it's as

- `&'a mut X` or `&'a mut [X]`, for a type parameter `X`;
- `&'a V`, where nothing in `V` can be changed through a `&` (no `Unsafe`,
  as in a `Cell`) but what's in its type parameters;
- inside enums, tuples, structs (their fields, as declared), boxes,
  arrays and slices of those.

And each such `X` must be *free*, as in step 55: bounded only by markers
(`Copy`, `Discard`, `Send`, `Sync`) and in no other bound. A bound can
make a `&mut X` out of a place that isn't one:

```rust,error
trait Pick {
    fn pick(n: &mut usize) -> &mut Self;
}

struct Counter<T> { n: usize, v: T }

impl<T: Pick> Counter<T> {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn slot(&mut self) -> &mut T ensures self.get() == old(self.get()) { T::pick(&mut self.n) }
}

fn main() {}
// error: `ensures` can't name `self` at return
```

With `T` a `usize` whose `pick` gives back what it's given, `*c.slot() =
99` would change `n`.

`List::index_mut`, `get_mut` (`Option<&mut T>`), `as_mut_slice` and
`slice_mut` (`&mut [T]`) and `iter_mut` (an `IterMut<T>`, a struct whose
field is a `&mut [T]`) all are, and say the length stays. So is a
program's own:

```rust
use alloc::list::List;

#[derive(Discard where T: Discard)]
struct Bag<T> { items: List<T>, label: u8 }

impl<T> Bag<T> {
    #[measure]
    fn size(&self) -> usize { self.items.len() }

    /// The `i`th item, to change; the size and the label stay.
    fn slot(&mut self, i: { i: usize | i < self.size() }) -> &mut T
    ensures
        self.size() == old(self.size()) && self.label == old(self.label),
    { &mut self.items[i] }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = Bag { items: List::filled(2, 'a'), label: 1 };
    if b.size() == 2 {
        *b.slot(0) = 'x';
        *b.slot(1) = 'y';
        out.line(b.slot(0))?; // expect: x
    }
    Ok(())
}
```

## What it can say

A postcondition about a lend-framed parameter can name, at return, what
the result can't change:

- a measure *free* in the type parameters the result can write: each of
  the measure's own type parameters that the parameter's type fills with
  one of them is free in the measure's impl. `size` of a `Bag<T>` is, as
  `impl<T> Bag<T>` bounds `T` by nothing. A measure that gives a `T: Int`
  isn't: its `T` isn't free.
- a field: it's an integer or a `bool`, never a free type parameter.

What the result can write isn't free:

```rust,error
struct Slot<T> { o: Option<T> }

impl<T> Slot<T> {
    #[measure]
    fn has(&self) -> bool { self.o.is_some() }

    fn whole(&mut self) -> &mut Option<T> ensures self.has() == old(self.has()) { &mut self.o }
}

fn main() {}
// error: `ensures` can't name `self` at return
```

The caller learns what's ensured when the borrow ends: that's when it
next reads the place, as the borrow checker keeps it out until then. It
learns it as of a call-scoped parameter (step 30): so a field a
refinement is about can be lent to such a method, and the refinement is
proved again of what it ensures, and a loop's frame (step 52) keeps what
it ensures stays.

```rust
use alloc::list::List;

#[derive(Discard)]
struct Table { width: usize, cells: { c: List<u32> | c.len() == width } }

impl Table {
    fn put(&mut self, i: { i: usize | i < self.width }, v: u32)
    ensures
        self.width == old(self.width),
    {
        self.cells[i] = v;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut t = Table { width: 2, cells: List::filled(2, 0) };
    t.put(1, 5);
    t.put(0, 4);
    out.line(&t.cells[1])?; // expect: 5
    Ok(())
}
```

## Methods on `x[i]`

`x[i]` is `index`'s, but where it's written to, `index_mut`'s; the parser
decided which, by the syntax around it (docs/21). A method's receiver
needs the method first, so `l[i].push(x)` took `index`, and couldn't
borrow `&mut` what that gave. Now the checker decides it there: if the
method takes `&mut self`, each `x[i]` its receiver is, or is in through
fields and `*`s, is `index_mut`'s.

```rust
use alloc::list::List;

#[derive(Discard)]
struct Row { items: List<u8> }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut rows: List<Row> = List::new();
    rows.push(Row { items: List::new() });
    rows.push(Row { items: List::new() });
    rows[1].items.push(7);
    rows[1].items.push(8);
    out.line(&rows[1].items.len())?; // expect: 2
    Ok(())
}
```

## Which variant

A measure can say which variant a value is now: its body can be a
`match` of `self`, or a field of it, whose patterns only say which
variant (`Some(_)`, `None`, `_`, alternatives with `|`) and whose arms
are terms of the logic. `Option`'s `is_some` and `is_none`, and
`Result`'s `is_ok` and `is_err`, are measures. So `List::pop` says it
gives a `Some` exactly when the list wasn't empty,

```text
export fn pop(&mut self) -> { r: Option<T> | r.is_some() == (old(self.len()) > 0) }
```

and a loop that pops until it can't knows what's left:

```rust
use alloc::list::List;

/// Moves every value of `from` onto `to`, the last first.
fn drain(from: &mut List<u8>, to: &mut List<u8>)
with {alloc, panic}
ensures
    to.len() == old(to.len()) + old(from.len()),
{
    let n = from.len() + to.len();
    while let Some(v) = from.pop() invariant from.len() + to.len() == n {
        to.push(v);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut a: List<u8> = List::new();
    a.push(1);
    a.push(2);
    let mut b: List<u8> = List::new();
    drain(&mut a, &mut b);
    out.line(&b[1])?; // expect: 1
    Ok(())
}
```

A pattern can't look at what's in a variant: the logic knows a variant's
fields only through refinements of its type arguments (step 53).

```rust,error
enum Shape { Dot, Line(u8) }

impl Shape {
    #[measure]
    fn long(&self) -> bool {
        match self {
            Shape::Dot => false,
            Shape::Line(n) => *n > 3,
        }
    }
}

fn main() {}
// error: a measure's `match` can only say which variant
```

## `swap`

`swap` of a slice or a list takes indices proved in range, as `s[i]`
does, and gives nothing back; it used to take any and give whether it
swapped. With the length kept through `as_mut_slice`, the proved one is
easy to call:

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u8> = List::filled(3, 0);
    l[2] = 9;
    l.swap(0, 2);
    out.line(&l[0])?; // expect: 9
    Ok(())
}
```

## A hole closed: a refinement proved from itself

A field a refinement is about can be lent to a call-scoped `&mut`
(step 30), and the refinement is proved again when the call returns. To
learn what the call ensures, the checker reads the field's new value, and
reading a new value of a field assumes the struct's refinements of it, as
every value of the struct has them. So when the callee ensured anything
at all, the refinement was assumed and then "proved" from that:

```rust,error
#[derive(Copy, Discard)]
struct Bag { extra: usize }

impl Bag {
    #[measure]
    fn more(&self) -> usize { self.extra }

    fn touch(&mut self) ensures self.extra >= 0 {
        self.extra = 7;
    }
}

#[derive(Copy, Discard)]
struct Holder { bag: { b: Bag | b.more() == 3 } }

fn first(h: &mut Holder) {
    h.bag.touch();
}

fn main() {}
// error: can't prove `b.more() == 3` for field `bag` of `Holder`
```

From 0.43.0 to 0.74.0 this was accepted, and a `Holder` whose `bag` had
`extra` 7 could be read; the interpreter caught it, the first time a
program here did it. What the call ensures is now read with nothing
assumed of the lent field's new value, and only then are the struct's
refinements proved, and then assumed.

## Why it's sound

- **Lemma L1 (references are parametric).** A function generic in a free
  `X` hands out a `&mut X` only to a place of type `X`: a field declared
  `X`, a slot of a buffer of `X`s, or what a `&mut X` it has points to.
  It can't turn a reference to anything else into one, as T has no
  transmute, no casts of references, and no way to look at what `X` is;
  markers make no values or references; and `unsafe` code that forged one
  would break its own obligation (as for lemma P1 of
  docs/55-free-theorems.md). A `&'a V` with no `Unsafe` in `V` but in its
  type parameters writes nothing else either (lemma U3 of
  docs/46-unsafe-cell.md). So whatever's written through the result, until
  its borrow ends, is values of type parameters in places of those types,
  inside what the parameter was lent.
- **Lemma L2 (measures are parametric).** A measure free in `X` has the
  same value of two values that differ only in their places of type `X`:
  Reynolds' abstraction theorem, with the relation that relates every two
  values of `X`, which markers respect. It can't compare, look into or make
  an `X`. A field the postcondition names is an integer or a `bool`, never
  of a free type parameter, so it's in no such place either.
- **Lemma L3 (true when the borrow ends).** The function proves its
  postcondition at return, of what's behind the parameter then. From then
  until the borrow ends, the borrow checker lets nothing else read or write
  the place, and what's written through the result is only in places of
  free type parameters (L1), which what's named can't see (L2): so it's
  true when the borrow ends. That's when the caller next reads the place,
  where it learns it: as for an intrinsic's prophecy (docs/30), but proved.
  A refinement of a field lent so is proved again of what's ensured, and
  holds whenever the struct can be read as a whole, which is after.
- **Lemma L4 (`index_mut` for receivers).** It asks the same of `i` as
  `index` does, and gives a `&mut` to the same place; the borrow checker
  checks the `&mut` borrow of the receiver, as for `x[i] = v`.
- **Lemma L5 (variant measures).** Which variant a value is is a function
  of the value (lemma E1 of docs/53), and the arms say a term of it for
  each: a measure as any other. The interpreter evaluates it at every
  refinement it checks.
- **The hole.** A struct's refinements are facts about every value of it
  that can be read; a lent field's value isn't one until they're proved
  again, so nothing about it may be assumed from them before. Reading it
  for the callee's postconditions now assumes nothing of it, which
  `err_soundness_post_field_assumed.t` tests.

## What it doesn't do

- A method that hands out a `&mut` to anything but a free type
  parameter's values still can't say what's left: `&mut self.len`, `&mut
  Option<T>`, a `&mut List<T>` of `self`.
- A trait's methods still can't have `ensures`, so an `IndexMut` trait
  would have nothing to say this with: `index_mut` is found by name.
- Nothing is known of a list's values: `l[0] = 5` doesn't tell the
  checker `l[0]` is 5 (step 60 is about refining them).
- A dead `match` arm is only found by a goal that shares a variable with
  what makes it dead (docs/55).

## Decisions

- D1, what results count: structurally, through enums, tuples, structs,
  boxes, arrays and slices, with shared references to what can't change
  through one; rather than only `&mut X`, `&mut [X]` and `Option`s of
  them, which would leave `iter_mut` and `split_at_mut` forgetting.
- D2, what can be named: measures free in what's written, and fields;
  rather than measures only.
- D3, `swap` takes proved indices and gives nothing back, replacing the
  one that checked: it's what `s[i]` asks, and a caller with indices it
  can't prove checks them first.
- Measures can say which variant, and `pop` says when it's `Some`.

## Exercises

1. `fn first_two(&mut self) -> (&mut T, &mut T)` on a `Bag<T>`: is it
   lend-framed? Write it with `split_at_mut`.
2. Why must the type parameters be free? Write the program the `Pick`
   example would let through, and say what the interpreter does with it.
3. `fn label_mut(&mut self) -> &mut u8` can't ensure `self.size() ==
   old(self.size())`, though `size` doesn't read `label`. What would the
   checker need to know to allow it? (Read about Rust's "view types".)
4. Give `Option` a measure `fn some_count(&self) -> u8` that's 1 for a
   `Some` and 0 for a `None`. What does `l.pop().some_count()` say?

## Reading

- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983), and
  Wadler, *Theorems for free!* (1989): L1 and L2 are free theorems.
- Matsushita, Tsukada and Kobayashi, *RustHorn: CHC-based Verification
  for Rust Programs* (2020), and Denis, Jourdan and Marché, *Creusot: a
  Foundry for the Deductive Verification of Rust Programs* (2022): a
  `&mut`'s final value as a prophecy, which they prove by passing it
  around; here the type alone says it.
- Lehmann, Geller, Vazou and Jhala, *Flux: Liquid Types for Rust* (2023):
  refinements kept through `&mut` borrows by their types.
- Matsakis, *View types for Rust* (blog, 2021): saying which fields a
  borrow of `self` reaches.
