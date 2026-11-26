# Step 100: chains, `..` and consts in patterns

Code: `Parser::condition` and `Parser::element` (`src/parser.rs`), which
read a chain of conditions and a `..` among a pattern's values;
`Rewrite::if_chain`, `Rewrite::nested` and `Rewrite::const_pattern`
(`src/check/modules.rs`), which say what a chain means and resolve a const
in a pattern; `Checker::pattern` (`src/check/patterns.rs`), which types
both; `ast::rest_fields`, the one place that says which value a pattern
beside a `..` is for; `Builder::const_values` (`src/borrowck/build.rs`),
which works out what a const pattern compares with; and `Pat::Opaque`
(`src/exhaust.rs`), what such a pattern covers.

Three things `t fmt` written in T wanted (see t-fmt's `FRICTION.md`), each
a way of saying in a pattern what took a `match` and a comparison before:

```rust
const CMP_PREC: u8 = 3;

#[derive(Discard)]
enum Expr { Num(u8), If(bool, u8, u8) }

fn tight(next: Option<(u8, u8)>) -> bool {
    if let Some((_, CMP_PREC)) = next { true } else { false }
}

fn is_if(e: &Expr) -> bool {
    match e {
        Expr::If(..) => true,
        Expr::Num(_) => false,
    }
}

fn small(o: Option<u8>) -> { v: u8 | v < 10 } {
    if let Some(n) = o && n < 10 { n } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&tight(Some((1, 3))))?;          // expect: true
    out.line(&is_if(&Expr::If(true, 1, 2)))?; // expect: true
    out.line(&is_if(&Expr::Num(7)))?;         // expect: false
    out.line(&small(Some(40)))?;              // expect: 0
    Ok(())
}
```

## Chains

The condition of an `if` or a `while` is a *chain*: conditions joined by
`&&`, each a `let` or a `bool`. They're worked out in order, each only
where the ones before it held; the block runs where they all do, and the
`else`, or the end of the loop, where one doesn't. What a `let` binds is
there in the conditions after it and in the block:

```rust
fn both(a: Option<u8>, b: Option<u8>) -> u8 {
    if let Some(x) = a && let Some(y) = b && x < 100 && y < 100 {
        x + y
    } else if let Some(x) = a && x < 200 {
        x + 1
    } else {
        7
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&both(Some(1), Some(2)))?; // expect: 3
    out.line(&both(Some(1), None))?;    // expect: 2
    let mut it = 0u8..10;
    let mut sum = 0u8;
    while let Some(i) = it.next() && i < 5 {
        sum = sum + i;
    }
    out.line(&sum)?; // expect: 10
    Ok(())
}
```

Each condition binds more tightly than `&&`, so `let Some(x) = a && c`
is `let Some(x) = a`, then `c`. A condition with no `let` in it is an
expression, as it always was, `a && b || c` and all. With one, `||` can't
join it: what the `let` binds would be there on one side and not the
other:

```text
if let Some(a) = o || c { a } else { 0 }
// syntax error: `||` can't join a condition with a `let` in it
```

What a chain means is written with what was there before (L3). A `let`
alone is the `match` it always was (docs/28-if-let.md). Several, with no
`else`, nest: each `let` a `match` whose other arm is `()`, each `bool` an
`if`. With an `else`, it's written once, after the conditions, so a chain
of `else if`s grows only as long as it's written, and what the block
gives is taken out of them in an `Option`:

```text
if let Some(x) = a && x < 10 { x } else { 0 }

match (match a { Some(x) => if x < 10 { Some(x) } else { None }, _ => None }) {
    Some(v) => v,
    None => 0,
}
```

So the block is where every condition holds, which is what the refinement
checker knows there, and the `else` is where one doesn't, of which it
knows nothing:

```rust,error
fn at(a: [u8; 4], o: Option<u8>, i: usize) -> u8 with panic {
    if let Some(_) = o && i < 4 { a[i] } else { a[i] }
}
// error: can't prove `i < self.len()`
```

A linear value a `let` takes is still there where a condition after it
doesn't hold, and nothing there uses it up. So it must be used up before
a condition that can fail, or after the last:

```rust,error
struct Fd { n: u8 }

fn close(fd: Fd) -> u8 {
    let Fd { n } = fd;
    n
}

fn f(o: Option<Fd>, c: bool) -> u8 {
    if let Some(fd) = o && c { close(fd) } else { 0 }
}
// error: `fd` isn't used up on every path
```

A `let` that always matches is worth one only between others, where it
names something for them, or first in a `while`'s, where it's worked out
again at each turn. First in an `if`'s, or last, it's a warning:

```rust
fn f(o: Option<u8>, n: u8) -> u8 {
    if let m = n && let Some(a) = o && let b = a + m && b > 3 { b } else { 0 }
    // warning: this pattern always matches, so this `let` needn't be one of the conditions
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(Some(2), 2))?; // expect: 4
    Ok(())
}
```

## `..` among a pattern's values

In a variant's, a tuple's or an array's pattern, `..` stands for the
values the others don't name, none or more: as many `_`s as there are
(P1). One at most, as with two, which values each stands for isn't said:

```rust
#[derive(Discard)]
enum Shape { Dot, Line(u8, u8), Box3(u8, u8, u8) }

fn last(s: &Shape) -> u8 {
    match s {
        Shape::Dot => 0,
        Shape::Line(.., z) | Shape::Box3(.., z) => *z,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&last(&Shape::Box3(1, 2, 3)))?; // expect: 3
    out.line(&last(&Shape::Line(4, 5)))?;    // expect: 5
    out.line(&last(&Shape::Dot))?;           // expect: 0
    let (x, .., y) = (1u8, 2u8, 3u8, 4u8);
    out.line(&(x + y))?; // expect: 5
    let [first, ..] = [5u8, 6, 7];
    out.line(&first)?; // expect: 5
    Ok(())
}
```

How many values that is is a variant's fields, a tuple's type or an
array's length, so it must be known where the pattern is. An array's
length must be a number (lemma A6 of docs/22-arrays.md), as `[x, ..]`
matches no array of none:

```rust,error
fn first<const N: usize>(a: [u8; N]) -> u8 {
    let [x, ..] = a;
    x
}
// error: this pattern is for an array of at least 1, but the value is `[u8; N]`
```

A struct's pattern already had its `..`, `P { x, .. }`, for the fields it
doesn't name, by name.

## Consts in patterns

A const, or a const parameter, in a pattern matches the value equal to it.
It's a path, as any name that starts with an uppercase letter is in a
pattern (lemma V3 of docs/45-variants.md): never a new variable. So the
reason a pattern couldn't name one, that in Rust a misspelt const binds
instead, is gone:

```rust
const LAST: usize = 3;
const KEYWORD: &str = "fn";

fn at_last(a: [u8; 4], i: usize) -> u8 {
    match i {
        LAST => a[i],
        _ => 0,
    }
}

fn word(s: &str) -> u8 {
    match s {
        KEYWORD => 1,
        _ => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&at_last([1, 2, 3, 4], 3))?; // expect: 4
    out.line(&word("fn"))?;               // expect: 1
    Ok(())
}
```

It's compared as a literal of its value would be, so its type must be one
a literal can be: an integer type, `bool`, `char` or `&str`. Their `==` is
the language's own, so matching runs no code of the program's. The arm
knows what `if i == LAST` would know, `i == 3` here, which is why `a[i]` is
in bounds: the logic knows a const's value if it can tell it from the
initializer (C5 of docs/36-consts.md).

What it compares with is worked out where the `match` starts, after what
it matches (P3). A const's value can't have an effect (C1 of docs/36), so
when it's worked out makes no difference to what happens.

The value isn't known while bodies are checked (docs/36), so a const
covers no value: a `match` on consts needs a `_`, even where they'd cover
everything (P2):

```rust,error
const YES: bool = true;
const NO: bool = false;

fn f(b: bool) -> u8 {
    match b {
        YES => 1,
        NO => 0,
    }
}
// error: non-exhaustive `match`: `_` not covered
```

An arm is unreachable, though, only where a `_` would be: after a `_`, a
const's arm is never reached, whatever its value.

## Messages

- A keyword where a name would be says it is one: "`sealed` is a keyword,
  so it can't be a name", where it said "expected a pattern, found
  `sealed`". Many of T's keywords are ordinary words (`with`, `effect`,
  `invariant`, `sealed`, `law`, `ensures`).
- Calling a parameter or a variable named as a function says which it is:
  "`head` is a parameter of type `Head`, not a function", where it said
  only "`Head` is not a function".
- A lifetime of a parameter whose type has several says which, marked
  `'1`: "the lifetime `'1` in the type of `p`, `&mut Parser<'1, '_>`",
  where it said "the lifetime of `p`".

## Why it's sound

- **L3 (chains).** A chain means the nested `match`es and `if`s above,
  each written with rules that are already sound (lemma L1 of step 28):
  the block runs exactly where every condition holds, with each `let`'s
  bindings from the value it matched; the `else` runs exactly where one
  doesn't, once, after them. What the block gives goes to the chain's
  value through `Some` and back out, moved, never copied or dropped. The
  names the rewrite makes up can't be written by a program.
- **P1 (`..`).** `..` is the `_`s it stands for, given their places by
  one function, `rest_fields`, for the checker, the exhaustiveness checker
  and the MIR alike. What a `_` does to a value it's in, `..` does: a
  linear value under one is thrown away, which is an error unless it's
  `Discard`.
- **P2 (coverage).** A const pattern may match any value or none, as far
  as checking knows, so it covers none when asking what's missing. Every
  value is then matched by some arm whatever the consts turn out to be,
  which is all exhaustiveness promises.
- **P3 (comparing).** The pattern matches a value exactly where it equals
  the const's, by a comparison the interpreter and the C program make on
  the value the const has when it runs. The refinement checker assumes
  that equality in the arm and its negation after, which is what the
  comparison decides, of the value the logic gives the const, which the
  interpreter checks is the one it has (docs/36).

## What it doesn't do

- **A const of a struct or an enum** can't be a pattern: that would need
  matching by structure, a rule of its own.
- **A const doesn't count as its value for exhaustiveness**, even where
  the logic knows it: then what's exhaustive would hang on how much the
  logic can tell.
- **A range of consts**, `LO..=HI`, isn't a pattern.
- **`let .. else` takes no chain.**
- **`..` in a slice's pattern**: T has no slice patterns.

## Decisions

- **D1: chains in `if` and `while`, as Rust 2024 has them**: `&&` only,
  and `let .. else` without one. Only `while let` (what t-fmt asked for)
  would have left an odd gap between `if` and `while`.
- **D2: `..` in a variant's, a tuple's and an array's pattern**, wherever
  the type says how many values there are. Only a variant's would have
  been enough for t-fmt.
- **D3: consts of an integer type, `bool`, `char` or `&str`, covering
  nothing**, rather than counting as their value where the logic knows it,
  or matching structs and enums by structure.

## Exercises

1. Write `if c && let Some(x) = o { x } else { 0 }` as the `match` and
   `if`s it means. Where is `c` worked out, and where `o`?
2. Why can't a chain with an `else` be written as nested `match`es with
   the `else` in each arm that doesn't match? What would a chain of five
   `else if`s grow to?
3. `let (a, ..) = t;` where `t: (Fd, Fd)` and `Fd` is linear. What does
   the error say, and where would `let (a, b) = t;` need `b` to go?
4. In `exhaust::missing`, let `known` keep what's `Opaque` as a `_`.
   Which test fails, and what would a program that passes then do?
5. Make `Builder::const_values` work a const out after the `match` looks
   at the value instead of before. What changes in what runs?

## Reading

- Rust RFC 2497, *if-let chains*, and the Rust reference, *`if let`
  expressions*: the chains here, and why `||` can't join a `let`.
- The Rust reference, *Rest patterns*: `..` in tuple, tuple-struct and
  slice patterns.
- Maranget, *Warnings for pattern matching* (2007): the usefulness
  algorithm `exhaust.rs` follows, here with a pattern that covers nothing
  for coverage and everything for reachability.
