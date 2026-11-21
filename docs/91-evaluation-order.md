# Step 91: the order operands are worked out in

Code: `Builder::operand_before`, `read_before` and `quiet` in
`src/borrowck/build.rs`, which read an operand where it's written; the
comparison arm of `expr_into_uncoerced` there, which reads or borrows a
comparison's left side; and, for statics as sites, `Builder::place` and
`Walk::assign_mir`'s `Rvalue::Use` (`src/check/refine/mir.rs`).

```rust
fn first(a: u8, _b: u8) -> { r: u8 | r == a } { a }

/// Makes `*c` 7, and gives 7.
fn seven(c: &mut u8) -> u8 {
    *c = 7;
    7
}

fn five(x: { x: u8 | x == 5 }) -> u8 { x }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut c = 5u8;
    out.line(&five(first(c, seven(&mut c))))?; // expect: 5
    out.line(&c)?;                             // expect: 7
    let mut d = 1u8;
    out.line(&(d + seven(&mut d)))?; // expect: 8
    let mut e = 5u8;
    let pair = (e, seven(&mut e));
    out.line(&pair.0)?; // expect: 5
    Ok(())
}
```

Operands are worked out left to right, each to its value before the next
is started. `first(c, seven(&mut c))` is given 5, what `c` was where it's
written, though its second argument makes `c` 7 before the call is made;
and the checker knows it's 5. A block does as a call does: `first(c, { c
= 7; 7 })` is given 5 too.

Until now that was so of everything but a variable that's `Copy`, or a
field of one, or what's behind a reference. Those were read where they
were used: by the call, once every argument was worked out. So `first`
was given 7, `d + seven(&mut d)` was 14, and the pair's first field 7. The
checker, the interpreter and the compiled program all read the same
instructions, so they agreed, and nothing was unsound: but `c` meant
something else than `c + 0` did in the same place, and no rule said so.
It was never decided; it was how a `Copy` place happened to be lowered.

## The rule

- **A call**: what's called first, then a receiver that's taken by value,
  then the arguments, left to right.
- **An operator**, `+` or `<` or `==`: its left side, then its right. `&&`
  and `||` work the right out only if the left doesn't decide them.
- **A tuple, a struct literal, an array, a variant**: its fields as
  they're written.
- **An assignment**: the value, then the place: `a[i] = f()` calls `f`
  before it looks for `a[i]`. `a += b` works `b` out and then is `a = a +
  b`, so `c += { c = 7; 1 }` makes `c` 8.

Each is worked out *to a value*: what's read of a variable is what it has
then. A value that isn't `Copy` is moved then, as it always was.

```rust
use alloc::list::List;

fn first(a: u8, _b: u8) -> u8 { a }

fn eat(p: (u8, List<u8>)) -> u8 {
    let (a, l) = p;
    l.discard();
    a
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let pair = (4u8, List::new());
    out.line(&first(pair.0, eat(pair)))?; // expect: 4
    Ok(())
}
```

`pair.0` is read before `pair` is moved into `eat`. Read at the call, it
was a use of a moved value, and this was refused.

## Borrows

A borrow that's passed to a call is not a value read where it's written:
it's taken when the call is made, after the arguments
(docs/06-traits.md). That's what lets `v.push(v.len())` be written. So a
reference argument is to its place as it is at the call:

```rust
fn through(a: &u8, _b: u8) -> u8 { *a }

fn seven(c: &mut u8) -> u8 {
    *c = 7;
    7
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x = 5u8;
    out.line(&through(&x, seven(&mut x)))?; // expect: 7
    Ok(())
}
```

Which place it is, is decided where it's written: a later argument can't
make `h.g` another reference's place (docs/83-sources.md). What's in it is what's there when the call is made.

A comparison looks at its operands where they are, without moving them.
Its left side is still what it was where it's written: read then, if it's
`Copy`; borrowed then, if it isn't, so that the right side can't change
it.

```rust,error
use alloc::list::List;

fn grown(a: &mut List<u8>) -> List<u8> with alloc {
    a.push(1);
    List::new()
}

fn main() with alloc {
    let mut a: List<u8> = List::new();
    let _same = a == grown(&mut a);
    a.discard();
}
// error: cannot borrow `a` as mutable while `a` is borrowed
```

## What's refused now

A closure that changes a variable borrows it from where it's made to
where it's last called. Reading the variable between the two was, and
is, refused; an argument before the closure's last call is now read
between them:

```rust,error
fn first(a: u8, _b: u8) -> u8 { a }

fn main() {
    let mut v = 5u8;
    let mut inc = || {
        v = 7;
        0u8
    };
    let _got = first(v, inc());
}
// error: cannot use `v` while `v` is mutably borrowed
```

And what the checker proved of a late read, it no longer does:

```rust,error
fn pair(a: u8, b: u8) -> u8 with panic if a != b {
    if a != b {
        panic("not the same");
    }
    a
}

fn seven(c: &mut u8) -> { r: u8 | r == 7 } ensures *c == 7 {
    *c = 7;
    7
}

fn safe(c: &mut u8) -> u8 with {} {
    match *c == 5 {
        true => pair(*c, seven(c)),
        false => 0,
    }
}

fn main() with {} {
    let mut c = 5u8;
    let _v = safe(&mut c);
}
// error: `pair` may panic, but `safe` doesn't say `with panic`
```

`pair` is given 5 and 7. Before, it was given 7 and 7, and this was
accepted, rightly then.

## Why it's right

- **Lemma E1 (read where written).** An operand that's a `Copy` place,
  with operands after it, is copied to a temporary where it's written,
  as one that isn't `Copy` is moved to one. Nothing changes a temporary
  but the statement that makes it, so the value the call, operator or
  literal uses is the place's where the operand is.
- **Lemma E2 (quiet).** The copy is left out when every operand after is
  *quiet*: a literal, a function's name, a `Copy` place read, a `&` of a
  place, or an operator on such. Working one of those out assigns
  nothing, moves nothing and calls nothing, so every place is, where the
  operands are used, what it was where the first was written, and still
  usable: reading it there is reading it earlier. A `&mut` of a place is
  quiet as a call's argument, where it's taken when the call is made, and
  not in a tuple or a struct, where it's taken where it's written, and
  what's behind it can't be read until it's used: so `(*r, r)` reads `*r`
  first. A
  comparison or an operator on a type that isn't an integer is a call of
  `eq` or `cmp`, with `&`s, which can't change a `Copy` place either.
  Saying an operand isn't quiet is always right: it costs a temporary.
- **Lemma E3 (one program).** The borrow checker, the refinement checker,
  the interpreter and the C backend all work from the same instructions
  (docs/32-mir.md). The rule is in how they're written, once, so
  what's checked is what runs: no rule of any of them changed.

## Statics are sites

A use of a static has the effect `global`, and step 90 let a function say
when it has it: `with global if counted`. But the refinement checker
didn't see a static's use, so it could never find one to be where the
condition holds, and such a function could only call others like it. Now
a static's use is a site as `Box::new` is: one whose trigger is `true`
(lemma C1 of docs/90-conditional-effects.md), which must be where the
condition holds.

```rust
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn step(counted: bool) -> u32 with global if counted {
    match counted {
        true => COUNT.fetch_add(1),
        false => 0,
    }
}

fn quiet() -> u32 with {} { step(false) }

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&quiet())?;    // expect: 0
    out.line(&step(true))?; // expect: 0
    out.line(&step(true))?; // expect: 1
    Ok(())
}
```

## What it doesn't do

- **Borrows passed to calls aren't taken in order.** `through(&x,
  seven(&mut x))` sees 7. Taking them where they're written would refuse
  `v.push(v.len())`; Rust takes a `&mut` receiver in two phases for that
  alone.
- **A comparison's right side isn't borrowed early**: nothing comes after
  it.
- **A condition beside an effect parameter is still refused** (docs/90):
  `with {I::e, panic if ..}` can't be written, so a function generic in
  an iterator can say when it panics only of one that has no effects,
  `I: Iterator<e = {}>`. The precise rule keeps, at a call, what the
  effect parameter was given, and takes out only what the condition put
  there.

## Decisions

- **D1: left to right, to values**, as Rust, C#, Java and Go. Keeping the
  late read and writing it down would have kept `c` and `c + 0` apart,
  and the refusal of `first(pair.0, eat(pair))`. Refusing a later operand
  that writes what an earlier one reads would have been right too, and
  needed an analysis of its own, to refuse what every other language
  runs.

## Exercises

1. What does `[i, { i = i + 1; i }, i]` give, for `i` 0? Which of its
   three operands are copied to temporaries?
2. `first(c, d)` copies nothing. Why is that the same as copying `c`
   first? What if `d` is `d + 1`? What if it's `g(d)`?
3. Why is a `&mut` variable quiet as a call's argument, and not as a
   tuple's field?
4. Write a program that the old order accepted and ran without a panic,
   and the new one accepts and panics in. Could the checker have been
   wrong about it before?
5. In `Builder::quiet`, say a block is quiet. Which tests fail?

## Reading

- The Rust Reference, *Evaluation order of operands*: left to right, with
  the receiver's two-phase borrow the one exception.
- The C standard's *sequence points*, and C++17's P0145, *Refining
  Expression Evaluation Order for Idiomatic C++*, for what leaving the
  order open costs.
- Matsakis, *Nested method calls via two-phase borrowing* (2017), on
  `v.push(v.len())`.
