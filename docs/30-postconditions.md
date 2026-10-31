# Step 30: what a call leaves behind a `&mut`

Code: `parser.rs` (`function`, `where_clause`), `check/modules.rs`
(`Rewrite::pred`), `check/refine.rs` (`call_scoped`, `Checker::scoped_params`,
`Env::Post`, `Walk::post`, `Walk::result`, `no_mut_borrow`,
`constrained_field`, `constrained_around`, `prove_fields`, `join`), and in
`interp.rs`, `Interp::refined_at`.

Until now a call that was lent a place `&mut` made the checker forget
everything it knew of the place. And a field a refinement is about
couldn't be lent `&mut` at all. Three things followed:

- `List`'s invariant, `len <= buf.cap()`, was a comment, as `buf` is lent
  to `grow`, `write` and `read`, so `push` checked for a `len` that
  can't count one more, which can't happen.
- A method couldn't say what it leaves as it was. The hash map's `put`
  doesn't change `len`, but nothing said so: a check before it was
  forgotten after.
- `l[i] = v` forgot `l.len()`, and so did `l.swap(i, j)`.

This step lets a function say what it does to what it's lent, and lets a
field a refinement is about be lent to a call that says.

## `ensures` and `old`

`ensures` comes after a function's `where`, if it has one. It's a list of
predicates, like a refinement's. In them a `&mut` parameter's name means
what's behind it when the function returns, and `old(..)` means what it
was at the call:

```rust
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    /// Counts one more, below `limit`.
    fn bump(&mut self, _limit: { l: usize | l > self.get() + 1 })
    ensures
        self.get() == old(self.get()) + 1,
    {
        self.n = self.n + 1;
    }
}

fn below(n: { n: usize | n < 3 }) -> usize { n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Counter { n: 0 };
    c.bump(5);
    c.bump(5);
    out.line(&below(c.get()))?; // expect: 2
    Result::Ok(())
}
```

The function proves what it ensures wherever it returns: at the end of
its body, at each `return`, and at each `?` that returns. The caller
learns it of what it lent, once the call returns: `c` is what `bump`
left, so `c.get()` is 2.

A result's refinement is a postcondition too, so it means the same: a
`&mut` parameter is its value at return, `old(..)` its value at the
call. Before this step it meant the value at the call, so step 21's
`take_last` now says `old`:

```rust
use alloc::list::List;

/// How long it was.
fn take_last(l: &mut List<i64>) -> { r: usize | r == old(l.len()) } {
    let n = l.len();
    l.pop();
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(5);
    l.push(6);
    out.line(&take_last(&mut l))?; // expect: 2
    out.line(&l.len())?;           // expect: 1
    Result::Ok(())
}
```

`old` can only be in a postcondition, and not inside another `old`,
which would mean the same.

## Call-scoped: when the value at return is the value after

What the caller learns is what the callee left behind the reference when
it returned. That's what the caller sees next only if nothing can change
it in between: if no reference the callee made from it outlives the
call. Then the parameter is *call-scoped*: its type is `&'a mut U`, where
`'a` is written nowhere else in the signature, not in `U`, another
parameter, the result or a bound. An elided lifetime is one. The callee
can't name anything that lives as long as `'a`, so it can't keep a
reference with that lifetime anywhere: not in its result, nor in what
another argument points to.

```rust
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }

    fn slot(&mut self) -> &mut usize ensures self.get() == old(self.get()) { &mut self.n }
}

fn main() {}
// error: `ensures` can't name `self` at return: the borrow it's lent can outlive the call, as the lifetime of `self` is in the signature elsewhere; write `old(..)`
```

`slot`'s result borrows `self`, so the caller can write through it
after `slot` returns. It's the same for an argument stashed in another
(`h: &mut Hold<'a>`), and for an `impl` result, which may hold every
lifetime of the signature. A postcondition can still say what such a
parameter was, with `old(..)`: `List::as_mut_slice` says its slice is
`old(self.len())` long.

## Invariants kept across a call

A field a refinement is about can now be lent `&mut`, straight to a
call-scoped parameter. When the call returns, the struct's refinements
about that field are proved again, as after an assignment to it, of what
the call left:

```rust
mod buf {
    #[derive(Copy, Discard)]
    pub struct Room { cap: usize }

    impl Room {
        pub fn new() -> Room { Room { cap: 0 } }

        #[measure]
        pub fn cap(&self) -> usize { self.cap }

        pub fn grow(&mut self, k: usize) with alloc ensures self.cap() == old(self.cap()) + k {
            if self.cap > core::num::greatest::<usize>() - k {
                alloc::raw::out_of_memory();
            }
            self.cap = self.cap + k;
        }
    }
}

use buf::Room;

#[derive(Copy, Discard)]
struct Stack { room: Room, len: { n: usize | n <= room.cap() } }

impl Stack {
    fn push(&mut self) with alloc {
        if self.len == self.room.cap() {
            self.room.grow(4);
        }
        self.len = self.len + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut s = Stack { room: Room::new(), len: 0 };
    s.push();
    s.push();
    out.line(&s.len)?;        // expect: 2
    out.line(&s.room.cap())?; // expect: 4
    Result::Ok(())
}
```

`self.room.grow(4)` lends `self.room`, which `len`'s refinement names.
After it, `len <= room.cap()` is proved of the new `room`: `grow`
ensures its `cap()` grew by 4, and before, `len == room.cap()`. And
`self.len + 1` can't overflow: `len < room.cap()`, which is a `usize`.

A call that doesn't say what it does gets nothing proved, so the call is
an error:

```rust
#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn zero(r: &mut i64) {
    *r = 0;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    zero(&mut p.n);
    out.line(&p.n)?;
    Result::Ok(())
}
// error: can't prove `v > 0` for field `n` of `Pos`
```

A `&mut` of such a field anywhere else is still an error: in a `let`,
or to a call that can keep it, where it might change after the checker
last sees it.

The same goes for a place *inside* such a field, through fields and
boxes: `s.a.b = 0` changes `s.a`, so if a refinement of `s` names
`s.a`'s measures, the assignment proves it again, and `&mut s.a.b` is
one of the field's. Before this step neither was checked, a hole that only
the interpreter caught. Not through a reference: what's behind `&` can't
be written, and no field's refinement can name what's behind a `&mut`.

## Intrinsics: what's true when the borrow ends

`List::as_mut_slice` lends `buf` to `RawBuf::slice_mut` for as long as
the slice lives, past the call. Its `ensures self.cap() == old(self.cap())`
is about `buf` when that borrow ends: a *prophecy*, in RustHorn's terms.
It's true because nothing done through a `&mut [T]` reaches the buffer's
length. Only an intrinsic can say such a thing, as the checker can't
prove it of a body; like an intrinsic's refinements, it's trusted, and
the interpreter checks it. The caller learns it of the place right away,
which is sound because the place isn't read again until the borrow
ends: the borrow checker says so.

So the library's own invariant is now a refinement:

```text
export struct List<T> { buf: RawBuf<T>, len: { n: usize | n <= buf.cap() } }
```

and `push` has no guard for a `len` that can't count one more: `len <
cap()` before the write, and `cap()` is a `usize`. `RawBuf`'s `write`,
`read`, `slice` and `slice_mut` now say `i < self.cap()` and `hi <=
self.cap()` as refinements, so that half of their safety conditions is
proved. What's left in their `# Safety` is which slots hold values.

## After a branch

A measure of an object is known only of that object, and a place each
branch of an `if` left a different object at was a new one after it, of
which nothing was known. Now each measure that every branch knows of its
object is known of the new one, case by case: here `l.len()` is 1 after
either branch, so `l[0]` is proved.

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    let first = l.is_empty();
    if first {
        l.push(1);
    } else {
        l.push(2);
        l.push(3);
    }
    out.line(&l[0])?; // expect: 1
    Result::Ok(())
}
```

## Why it's sound

Step 21's induction, with three more ways to learn facts.

- **P1 (call-scoped).** No reference made from a call-scoped parameter
  outlives the call. The callee can only keep one in a place whose type
  says how long it lives, and `'a` outlives nothing the callee can name,
  nor is outlived by it (no bound says so), nor is it in a type argument,
  which the caller picks; an `impl` result, which might hold it, rules
  the parameter out. So when the callee returns, nothing reaches what's
  behind the reference but the caller's place, which the borrow checker
  kept from everything else during the call.
- **P2 (the callee's proof).** At a return, every borrow the body made
  has ended but those that flow into the result, and none of the
  parameter's do (P1). So reading `*x` there, with step 21's "forget
  where lent", reads the value the caller will see.
- **P3 (the caller's fact).** After the call, the lent place is a new
  object, whose value is what the callee left (P1), of which the
  postcondition holds (P2). `old(..)` is about the arguments as they
  were, kept from before the call.
- **P4 (invariants across a lend).** A struct's refinements hold of
  every value of it whenever it can be read. During a call lent one of
  its fields, nothing can read it: the caller waits, the borrow is
  exclusive, and arguments are evaluated before. A panic or running out
  of memory aborts, and nothing unwinds into a struct with its refinement
  broken. When the call returns, they're proved again, of the fields as
  they were before (which had them) and what the call left in the lent
  one, without assuming them (`Walk::proving_fields`).
- **P5 (prophecies).** An intrinsic's `ensures` about a `&mut` whose
  borrow outlives the call holds of the value when it ends. The caller's
  place isn't read before then, so the new object stands for that value.
- **P6 (joins).** A measure is a function of a value (M1), so of the
  value a branch left, the measure is what that branch knew of it.

The interpreter checks what the checker can't see: each `ensures` and
result refinement when the call returns, with `old(..)`'s measures
worked out at the call, and an intrinsic's too.

## Design decisions worth noticing

- **One meaning for a postcondition.** A name in a result's refinement
  and in `ensures` means the same: the value at return. Leaving result
  refinements at the call, as step 21 had them, would have given a name
  two meanings in two places.
- **Which parameters, by their lifetimes.** "Call-scoped" is read off
  the signature, so callers don't depend on a body. Changing a signature
  so that a lifetime is in two places is a breaking change already.
- **Re-prove, don't pack.** Spec#'s methodology has an object
  *unpacked* while its invariant may be broken, with `pack` and `unpack`
  written in the program. T needs none of it: an invariant is broken
  only during a call lent the field, which the borrow checker keeps
  everyone else out of, and is proved again at the call's end.
- **Prophecies for intrinsics only, for now.** A function whose borrow
  outlives the call can't prove what's true when it ends: the caller
  decides when, and does what it likes until then. RustHorn and Creusot
  prove such things by passing a prophecy variable around; an
  intrinsic's body is trusted anyway.
- **`RawBuf` keeps `&mut`.** Writing slots through a `&RawBuf` would have
  let `List` lend `buf` only shared, with no prophecy. But `RawBuf<T>` is
  covariant, so a `&RawBuf<&'long T>` is a `&RawBuf<&'short T>`, through
  which a short-lived reference could be stored where a long-lived one is
  read: why Rust's `Cell` is invariant.
  (`tests/t/err_soundness_rawbuf_lifetime.t` caught it.)

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `ensures` on what a borrow that outlives the call leaves, in a program's own functions (`index_mut` keeps `len()`, so `l[i] = v` too) | prophecies the callee can prove (RustHorn, Creusot) | — |
| `ensures` on a trait's methods | an impl's would have to promise no less | — |
| what a closure ensures | closures have no signatures to write it in | — |
| a map's `len` is the sum of its buckets' | sums over a list, or quantifiers, in the logic | — |
| `*r` in a predicate, for an `r: &mut i64` | dereferences in the logic, as measures of a reference | — |

## Exercises

1. Give `List::truncate` an `ensures` that also says it didn't grow the
   list. What does `clear` then prove?
2. Write a function with two `&mut` parameters that share a lifetime.
   Which of them can `ensures` name? Why not the other?
3. Remove `ensures` from `RawBuf::write`. Which method of `List` fails,
   and at what?
4. In the hash map (<https://github.com/lkurcak/t-hashmap>), `put` says it
   keeps `len()`. Take that out: what does `insert` stop proving?
5. Lemma P4 needs that nothing unwinds. What would go wrong if a panic in
   `grow` could be caught by the caller of `push`?

## Reading

- Yusuke Matsushita, Takeshi Tsukada and Naoki Kobayashi, *RustHorn:
  CHC-based Verification for Rust Programs* (2020): prophecies, the value
  a mutable borrow has when it ends.
- Xavier Denis, Jacques-Henri Jourdan and Claude Marché, *Creusot: a
  Foundry for the Deductive Verification of Rust Programs* (2022), whose
  `^x` is the final value of a borrow.
- Vytautas Astrauskas, Peter Müller, Federico Poli and Alexander J.
  Summers, *Leveraging Rust Types for Modular Specification and
  Verification* (Prusti, 2019): `old(..)`, and postconditions on what
  `&mut` arguments are left as.
- Mike Barnett, Robert DeLine, Manuel Fähndrich, K. Rustan M. Leino and
  Wolfram Schulte, *Verification of Object-Oriented Programs with
  Invariants* (2004): when an invariant may be broken, and `pack`.
- Nico Lehmann, Adam T. Geller, Niki Vazou and Ranjit Jhala, *Flux:
  Liquid Types for Rust* (2023): strong references, a `&mut` whose type
  says what it's left as.
