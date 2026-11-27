# Step 86: trait contracts

Code: `Checker::inherit_contracts` and `Checker::inherit`, `asks_no_more`
(`src/check/refine/mir.rs`), `Checker::impl_measures`,
`Checker::measure_loops` and `Checker::trait_measure`
(`src/check/refine.rs`); in the walk, `Walk::pred_measure` and
`Walk::resolve_measure`; `Mono::measure_copy` and `Mono::contract` in
`src/mono.rs`, and the inherited contract in `Interp::call`
(`src/interp.rs`); the signature of `io::Read::read` in `core/io.t`.

A trait's method couldn't say what it needs or what it gives: refinements
and `ensures` were refused there, as was `#[measure]`. So a trait could
only promise types, and code generic in it knew nothing more: `Read::read`
gives a count of bytes, which every reader keeps under `buf.len()`, but
nothing generic could count on it, and `read_to_end` checked again.

Now a trait's method has a *contract*, as a function does: what it asks
of its parameters, and what it promises of its result and of what's behind
its `&mut`s. Each impl's method has it too, whether or not it says it
again.

```rust
trait Small {
    /// Something under 10.
    fn get(&self) -> { v: usize | v < 10 };
}

#[derive(Discard)]
struct Nine {}

// Says nothing more: its `get` gives what `Small`'s says, and its body is
// checked to.
impl Small for Nine {
    fn get(&self) -> usize { 9 }
}

// Generic code counts on it.
fn index<T: Small>(t: &T) -> u8 {
    let a = [5u8; 10];
    a[t.get()]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&index(&Nine {}))?; // expect: 5
    Ok(())
}
```

## What an impl's method has

What its trait's method asks of a parameter, the impl's asks too, unless
it says what it asks itself; what the trait's promises, the impl's
promises, and anything more it says besides:

- **Asking less.** An impl's method may refine a parameter, if what the
  trait's asks is enough for it: that's proved where the impl is, from
  what the trait's method asks of all its parameters. A call through the
  trait proves only the trait's, so the impl's mustn't need more.
- **Promising more.** What the impl's result and `ensures` say adds to
  the trait's. A call known to be of the impl, as `n.get()` of a `Nine`
  is, learns both; a call through a bound learns only the trait's.
- Its body is checked with what it asks, and must keep every promise: the
  trait's too, for everything it accepts.
- **No `mut` where a promise looks.** What the trait's method promises is
  of its parameters' values as passed, so the impl's can't make one it
  names `mut` (`mut self` too), which could be another by return: as a
  function's own refinements can't name a `mut` parameter. It copies it
  instead, `let mut k = k;`. A parameter that only what the trait's asks
  names may be `mut`: what it asks is of the value passed, and each value
  given to a refined one must keep its refinement.

```rust
trait Pick {
    fn pick(&self, i: { i: usize | i < 3 }) -> { v: u8 | v < 10 };
}

#[derive(Discard)]
struct Three { a: [{ v: u8 | v < 10 }; 3] }

impl Pick for Three {
    // Takes 3 too, which gives 0: a call of `Three`'s own may pass it.
    fn pick(&self, i: { i: usize | i <= 3 }) -> { v: u8 | i < 3 || v == 0 } {
        if i < 3 { self.a[i] } else { 0 }
    }
}

fn through<P: Pick>(p: &P) -> { v: u8 | v < 10 } { p.pick(2) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = Three { a: [4, 5, 6] };
    let z: { v: u8 | v == 0 } = t.pick(3);
    out.line(&z)?;           // expect: 0
    out.line(&through(&t))?; // expect: 6
    Ok(())
}
```

```rust
trait Below {
    fn below(&self, k: usize) -> { n: usize | n < k || k == 0 };
}

#[derive(Discard)]
struct Half {}

impl Below for Half {
    fn below(&self, mut k: usize) -> usize {
        k = k / 2;
        k
    }
}

fn main() {}
// error: `k` can't be `mut` in `below`: what trait `Below` promises names it, as it was passed; write `let mut k = k;` in the body
```

An impl's method that asks more than its trait's is refused where it's
written, not at some call:

```rust
trait Half {
    fn half(self, by: i64) -> i64;
}

impl Half for i64 {
    fn half(self, by: { b: i64 | b > 0 }) -> i64 { self / by }
}

fn main() {}
// error: can't prove `b > 0` from what trait `Half` asks of `by`
```

A trait's default body is checked once, generically, with what the
trait's method asks, and must keep its promises; an impl that doesn't
write its own has it, contract and all.

## Measures in traits

A trait's method can be a measure: `#[measure] fn len(&self) -> usize;`.
Each impl's must be one too, marked `#[measure]`, with a measure's body
(docs/21). A trait may give a default, a measure of `Self`'s other
measures, as `#[measure] fn is_empty(&self) -> bool { self.len() == 0 }`.
A default is final: it's what the measure is of every type, which generic
code counts on, so no impl can give its own, even one saying the same.

A predicate names one as any measure, `s.size()`. Of a value whose type
is known, it's that type's impl's: its body, where that can be seen, as
an inherent measure's is. Of a value of a type parameter, it's what's
been learned of it: generic code knows of `s.size()` only what the
trait's contracts say, as of any measure whose body it can't see.

```rust
use alloc::list::List;

trait Stack {
    type Item;

    #[measure]
    fn size(&self) -> usize;

    fn push(&mut self, v: Self::Item) with alloc ensures self.size() == old(self.size()) + 1;
}

#[derive(Discard)]
struct Bag { l: List<u8> }

impl Stack for Bag {
    type Item = u8;

    #[measure]
    fn size(&self) -> usize { self.l.len() }

    fn push(&mut self, v: u8) with alloc {
        self.l.push(v);
    }
}

fn two<S: Stack>(s: &mut S, a: S::Item, b: S::Item) -> { n: usize | n >= 2 } with alloc {
    s.push(a);
    s.push(b);
    s.size()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b = Bag { l: List::new() };
    out.line(&two(&mut b, 1, 2))?; // expect: 2
    Ok(())
}
```

A measure is a function of the value, so its body can't read through a
`&mut` (but for a slice's or string's length, which is the reference's):
what's behind one can change while the value holding it, an address,
stays the same. Nor can an impl's measures call each other, of `self`,
round in a circle, through the trait's defaults or its own: `size` calling
`self.is_empty()`, whose default calls `self.size()`, would never end. A
measure of a field is of a smaller value, which does.

## The library

`io::Read::read` says it reads no more than there's room for:

```text
fn read(&mut self, buf: &mut [u8]) -> Result<{ n: usize | n <= buf.len() }, Error> with Self::e;
```

`Bytes`'s and `&mut R`'s keep it as they are; the standard input's and a
file's check what the operating system says (which keeps it), as an
`Other` error if it ever didn't, as no new trust is needed for it.
`alloc::io::read_to_end` no longer checks `i < chunk.len()` again.

## Why it's sound

- **B1 (behavioural subtyping).** A call through a bound `T: Tr` proves
  the trait's method's precondition, at `T`, and learns its
  postcondition. Monomorphisation runs some impl's method, at `T = X`
  (static dispatch). That method's body was checked with its own
  precondition, which the trait's implies (proved at the impl), and to
  keep the trait's promises, at `X`, as well as its own. A predicate is a
  function of the values it names and of their measures, and a trait's
  measure at `X` is `X`'s impl's (B2): so the trait's precondition at `T`
  is the trait's at `X` is enough for the impl's, and what the body
  promised at `X` is what the call learned at `T`. A default body is the
  same, checked at `Self` itself.
- **B2 (trait measures are functions of values).** Each impl's measure is
  a measure, total and of the value alone, no `&mut` read through (H1);
  which one a trait's is at a type depends on the type alone (one impl
  per type, by the overlap rules), as the walk and monomorphisation both
  find it. In generic code it's unknown, so what's known of it is only
  what was proved or promised of the value, as for a measure whose body
  is hidden (M2 of docs/21), but for a default, which generic code
  unfolds: so a default must be what the measure is at every type, which
  it is as no impl can give its own (`impl_measures`). Every measure ends:
  on a smaller value, or through a chain of measures of `self` with no
  circle, at the impl's type (`measure_loops`).
- **B3 (modularity).** An impl's obligations need only its trait's
  signature, contract included: adding an impl, or a package, proves
  what it must where it's written, and changes nothing elsewhere.
- **B4 (the oracle).** The interpreter checks, at each call of an impl's
  method, the trait's method's contract as well as the impl's: what it
  asks of each parameter the impl's doesn't refine, and what it promises
  at return. A trait's measure is evaluated as the impl's, at the copy's
  types, which monomorphisation finds.
- **H1 (a hole closed).** Since measures (step 21), a measure could read
  through a `&mut` field, `self.r.len()`. Outside its module it's known
  by the value's object, which a write through `m.r` doesn't renew (a
  reference is the same one whatever is written through it, V2 of
  docs/85), so `m.n() == 0` stayed known after `m.r.push(1)`. Refused
  now (`err_soundness_measure_through_mut.t`), found while planning this
  step, as an impl's measure for `&mut I` would have been the same.
- **H2 and H3 (holes closed in 0.101.1).** Retrospective 15 found two in
  0.101.0, where inheriting a contract didn't keep what it rests on. An
  impl could give its own body for a default measure, `is_empty` true of a
  value with `len` 5, while generic code unfolded the default
  (`err_soundness_trait_measure_default*.t`); and an impl's method could
  make a parameter its trait's promise names `mut`, and keep the promise
  of the value it set, not the one passed
  (`err_soundness_trait_mut_*.t`). B1 needs both: what the impl's body
  promised at `X` is what a call learned at `T` only if the predicates
  mean the same in both, the measures the same and the names the same
  values. `tests/generated/inherit.rs` varies what an impl may write
  differently from its trait's, with the interpreter as the oracle.

## What it doesn't do

- **Laws about a supertrait's methods.** `len` for an iterator would be a
  measure of a trait `ExactSize: Iterator`, saying how `Iterator::next`
  changes it: a contract on another trait's method, which `Rev` needs of
  `next_back` too. So adapters still have no `len`, and `count` and
  `enumerate` still say `with panic` (docs/84). (Step 88: laws,
  docs/88-iterator-laws.md. Steps 92 and 96: neither panics of a bounded
  iterator, docs/96-associated-effects-unless.md.)
- A trait's measure can't be implemented for a slice or `str` (which only
  the built-in libraries can implement traits for): its measures are of
  the reference.
- A measure of a trait that's one of several traits a type implements,
  with the same name, can't be named in a predicate.
- A blanket impl through a reference, `impl<'a, T: Tr> Tr for &'a T`,
  can't give a measure of the trait's: a measure's body can't write `*`,
  and `self.size()` there would be its own (through a `&mut`, it couldn't
  read anyway, H1).
- An impl's method can't promise less of what's behind a `&mut` than its
  trait's does, at return, if what it gives back may still borrow it.

## Decisions

- **D1: entailment, with inheritance.** An impl's method has its trait's
  contract without writing it; what it writes of a parameter must follow
  from the trait's, and what it promises adds to the trait's. Restating
  the trait's contract at every impl was the alternative, as Rust does
  types: explicit, but long, and checked by comparing predicates as text.
- **D2: `#[measure]` on both.** A reader of an impl sees its method is
  held to a measure's rules.
- **D3: `Read` and t-life's `Engine::board` now; iterator laws later**,
  with their own design.
- **D4: refuse reading through a `&mut` in a measure.** Following what a
  write through a `&mut` changes, up to every value holding it, is more
  machinery, for measures no program has written.
- **D5 (0.101.1): a default measure is final.** Proving an impl's own
  body equal to the default would let one be written for speed alone;
  hiding defaults from generic code would lose `is_empty` meaning
  `len() == 0` there.
- **D6 (0.101.1): no `mut` parameter a trait's promise names**, the rule
  a function's own signature has. Letting the walk keep each parameter's
  value as passed, for what a signature names, would be more machinery,
  for what `let mut k = k;` does.

## Exercises

1. Give `Stack` a `pop` whose result is `Some` exactly when `size` was
   more than 0. Which of `Bag`'s methods must change?
2. An impl's `pick` asks `i <= 3` where the trait's asks `i < 3`. Why may
   its body still not index `a[i]`? What would it have to write?
3. `trait_contract_blanket.t` forwards `Small::get` through `&T`. Why
   needn't the blanket impl say anything? What if it added `1`?
4. Why is `#[measure] fn n(&self) -> usize { self.r.len() }` refused for a
   `r: &'a mut List<u8>`, and not for `r: &'a List<u8>`, nor for a slice's
   `rest: &'a mut [T]`?
5. Write a trait whose default measure calls another, and an impl whose
   measure calls the default, of `self`. What does the checker say? What
   if it calls it of a field?
6. An impl's method may take `mut v` where its trait's asks
   `v: { v: usize | v <= hi }`, but not `mut hi` where its trait's
   promises `{ n: usize | n <= hi }`. Why the difference? Is `mut hi`
   refused where only the trait's asks name it?

## Reading

- Liskov and Wing, *A Behavioral Notion of Subtyping* (1994): an impl's
  method as a subtype of its trait's, precondition weaker, postcondition
  stronger.
- America, *Designing an Object-Oriented Programming Language with
  Behavioural Subtyping* (1991).
- Findler and Felleisen, *Contract Soundness for Object-Oriented
  Languages* (2001), on checking an override against what it overrides.
- Leino, *Dafny: An Automatic Program Verifier* (2010), traits with
  specifications their classes refine; Verus's trait method specs, which
  impls inherit.
- Leavens and Naumann, *Behavioral Subtyping, Specification Inheritance,
  and Modular Reasoning* (2015).
