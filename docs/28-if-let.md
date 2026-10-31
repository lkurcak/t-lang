# Step 28: `if let`, `let .. else` and `%`

Code: `parser.rs` (`if_expr`, `let_stmt`, `let_cond`, `no_chain`),
`check/modules.rs` (`Rewrite::if_let`, `while_let`, `let_else`,
`else_pattern`), `check/refine.rs` (`remainder`), and the messages in
`check/patterns.rs`, `check/expr.rs` and `borrowck/moves.rs`.

The first library written in T outside this repository, a hash map
(<https://github.com/lkurcak/t-hashmap>), had four places that do
something only if a value is there, each a `match` with an arm that does
nothing:

```rust
fn bump(o: Option<&mut i64>) {
    match o {
        Option::Some(v) => *v = *v + 1,
        Option::None => {},
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n = 1;
    bump(Option::Some(&mut n));
    bump(Option::None);
    out.line(&n)?; // expect: 2
    Result::Ok(())
}
```

`if let` says it without the empty arm:

```rust
fn bump(o: Option<&mut i64>) {
    if let Option::Some(v) = o {
        *v = *v + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n = 1;
    bump(Option::Some(&mut n));
    out.line(&n)?; // expect: 2
    Result::Ok(())
}
```

This step adds `if let` (and `else if let`), `while let` and `let ..
else`, each as the `match` it means, so with no typing rule of its own;
and a fact about `%` by any divisor, which the map wanted for its buckets.

## `if let`, `while let` and `let .. else`

`if let pat = e { .. }` runs its block if `pat` matches `e`, with what
`pat` binds; an `else` runs if it doesn't, and can be another `if` or `if
let`. `while let` runs its body as long as the pattern matches, `break`
and `continue` and labels working as in a `while`:

```rust
#[derive(Discard, Copy)]
enum Shape { Circle(i64), Rect(i64, i64), Empty }

fn area(s: Shape) -> i64 {
    if let Shape::Circle(r) = s { 3 * r * r } else if let Shape::Rect(w, h) = s { w * h } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let all = [Shape::Circle(1), Shape::Rect(2, 3), Shape::Empty];
    let mut shapes = all.iter();
    let mut total = 0;
    while let Option::Some(s) = shapes.next() {
        total = total + area(*s);
    }
    out.line(&total)?; // expect: 9
    Result::Ok(())
}
```

`let pat = e else { .. };` binds what `pat` does for the rest of the
block, if it matches. If it doesn't, the `else` runs, and it must leave the
block: `return`, `break`, `continue`, or call a function that returns `!`.
It's the early return of tcalc's tokenizer:

```rust
fn digit(c: char) -> Option<i64> {
    match c {
        '0'..='9' => Option::Some((c as u32 - 0x30) as i64),
        _ => Option::None,
    }
}

fn first_digit(s: &str) -> i64 {
    let Option::Some(c) = s.chars().next() else { return -1 };
    let Option::Some(d) = digit(c) else { return -2 };
    d
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&first_digit("7up"))?; // expect: 7
    out.line(&first_digit(""))?;    // expect: -1
    out.line(&first_digit("up"))?;  // expect: -2
    Result::Ok(())
}
```

## Each is a `match`

> **Lemma L1.** `if let p = e { a } else { b }` means `match e { p => a,
> q => b }`, where `q` matches what `p` doesn't; with no `else`, `b` is
> `()`. `'l: while let p = e { body }` means `'l: loop { match e { p =>
> body, q => break } }`.

> **Lemma L2.** In a block, `let p = e else { d }; rest` means
>
> ```text
> match e {
>     p => { rest },
>     q => { let _: ! = { d }; },
> }
> ```

They're rewritten so as paths are resolved (`check/modules.rs`), the way
`for` (docs/06e-core.md) and `?` (docs/15-try.md) are, and nothing after
that knows they were written. So nothing about them needs a rule:

- The types are T-Match's (docs/02-adts.md). `if let` without `else` is a
  `()`, as an `if` without `else` is; with one, both have one type.
- `p`'s variables are in `a` or `rest`, and not in `b` or `d`: they're
  only in the first arm. What `rest` declares is gone after the block, as
  it would be.
- `d` is a `!`, so it can't finish: the `match` is `rest`'s type, which the
  block was. That's the rule that the `else` leaves the block, said as a
  type, and the error for one that doesn't comes from that `let`:

```rust,error
fn f(o: Option<i64>) -> i64 {
    let Option::Some(n) = o else { 0 };
    n
}
// error: the `else` of a `let .. else` must not finish, but this one can, as a `{integer}`
```

- `e` is evaluated once, before either arm, as in `let`. A `break` or
  `continue` in `d` is for the loop it would be, as the rewrite adds no
  loop; a `while let`'s body can't `break` with a value, as a `while`'s
  can't, since it can end without one.
- What a pattern says of a number holds in its arm (docs/08), so `let
  1..=9 = n else { return };` tells the checker `1 <= n <= 9` after it.

A nested `fn` or type after a `let .. else` stays in the block it was in,
where it means the same anywhere, since it can't use the variables around
it.

## The `else` and linear values

What should `q`, the arm that's left out, be? `_` matches what `p`
doesn't, but it doesn't take the value apart, so a linear value matched in
it is left unused (docs/06a.1-linear-types.md). That would reject one of
the most useful cases:

```rust
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn close_any(o: Option<Fd>) -> i64 {
    if let Option::Some(fd) = o { close(fd) } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(close_any(Option::Some(Fd { n: 3 })) + close_any(Option::None)))?; // expect: 3
    Result::Ok(())
}
```

So `q` is made of the variants `p` misses, each with `_` for its fields:
here `Option::None`. If `p` looks inside its own variant, as
`Option::Some(Shape::Circle(r))` does, that variant is in `q` too, as
`Option::Some(_)`.

> **Lemma L3.** `q` matches exactly the values `p` doesn't, and takes them
> apart to the variant they are, throwing away only their fields, which no
> code can name.

A variant pattern uses up what it matches, and what's under its `_`s is
thrown away, which only a `Discard` value can be (docs/06a.1). So the
`else` gets the value apart as far as `p` looks, and what it throws away
is what's really lost:

```rust,error
struct Fd { n: i64 }

fn f(r: Result<i64, Fd>) -> i64 {
    if let Result::Ok(n) = r { n } else { 0 }
}
// error: if the pattern of this `if let` doesn't match, it throws away `r.0`
```

The `Fd` in an `Err` can't be named in the `else`, so it would be thrown
away. To use it up, write the `match`.

`q` is built from the enum `p` names, as the module sees it. A local
enum's or an alias's variants aren't known there, and then `q` is `_`:
that only rejects more, for linear values. If `q` were ever wrong, the
`match` wouldn't be exhaustive, and that's an error too: neither choice
can let an unsound program through, which is why one built from names is
safe.

A pattern that always matches makes the `else` dead, and gets a warning,
as an unreachable `match` arm does:

```rust
#[derive(Discard, Copy)]
struct Point { x: i64, y: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = Point { x: 1, y: 2 };
    let Point { x, y } = p else { return Result::Ok(()) };
    out.line(&(x + y))?; // expect: 3
    Result::Ok(())
}
// warning: this pattern always matches, so the `else` never runs: use `let`
```

## The syntax

- `if let p = e` and `while let p = e` take what a condition takes; `e`
  can't be a struct literal without parentheses, as in `if`.
- An `&&` or `||` at the top of what's matched is an error, `if let
  Option::Some(n) = o && n > 0`: that's a *let chain* in Rust, and may
  mean that here one day. In parentheses, it's a `bool`, matched.
- What's before the `else` of a `let .. else` can't end in `}`: in `let x
  = if c { 1 } else { 2 } else { return };`, which `else` is whose? As in
  Rust, write the parentheses.
- A `let .. else` has no type: `Option::<i64>::Some(x)` says it, if it's
  needed.

`t fmt` lays out `if let` as `if`, and `let .. else` as `let` with a block
after it, on one line if it's an expression that fits:

```rust,fragment
let o = Option::Some(1i64);
let Option::Some(x) = o else { return };
let Option::Some(y) = o else {
    let _ = x;
    return;
};
```

## What `%` gives

The map's bucket for a hash `h` is `h % n`, for `n` buckets, and indexing
needs it less than `n`. The checker knew what `%` by a constant gives
(docs/08), but `n` isn't one, and `%` by a variable isn't linear: it knew
nothing, so the map used `h & (n - 1)` instead. Now it knows this of every
`a % b`:

> **Lemma L4.** If `b > 0`, `a % b` is from `0` to `b - 1` and at most `a`
> if `a >= 0`, and from `1 - b` to `0` and at least `a` if `a <= 0`; if `b <
> 0`, the same with `-b`. For every integer type.

```rust
fn slot(h: u64, n: { n: u64 | n > 0 }) -> { i: u64 | i < n } { h % n }

fn rem(a: i64, b: { b: i64 | b > 0 }) -> { r: i64 | -b < r && r < b } { a % b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&slot(17, 5))?; // expect: 2
    out.line(&rem(-7, 3))?;  // expect: -1
    Result::Ok(())
}
```

Why it's true of every width: `%` rounds towards zero, as in Rust, so
over the integers `r = a - b·trunc(a / b)` is less than `|b|` from 0, no
further from 0 than `a`, and 0 or of `a`'s sign. Those are the four facts,
each linear in `a`, `b` and `r`, so the solver can use them. The machine's
`%` gives that integer whenever it doesn't fail: an unsigned type's values
are the naturals it can hold, and a signed one's remainder is less than
`|b|` from 0, so it fits whatever `b` fits. The one `%` that fails, the
least value by `-1` (whose quotient doesn't fit), is proved not to happen
before the fact is known, and its remainder, 0, would satisfy it anyway.
Nothing in the facts mentions a width, so they hold of `T: Int` too.

What it doesn't say: that `r` can be `b - 1`, or `a`, which it can, so
they're the tightest bounds.

```rust,error
fn rem(a: i64, b: { b: i64 | b > 0 }) -> { r: i64 | r >= 0 } { a % b }
// error: can't prove `r >= 0` for the result of `rem`
```

## Design decisions worth noticing

- **A rewrite, not rules.** Every rule the forms need is one T has: `if
  let` can't be unsound in a way `match` isn't. The cost is messages, which
  the made-up arm's position, the `let`, lets the checker word in terms of
  what was written.
- **`else` arms that take values apart.** The arm that's left out matches
  the other variants, not `_`, so a linear value is used up in the `else`
  and only what can't be named must be `Discard`.
- **`let .. else` as a `!`.** "The `else` must leave the block" is the
  type `!`, and divergence tracking (docs/05.1) decides it.
- **Room for chains.** `&&` and `||` are errors where a chain would go, so
  adding chains later changes no program that checks now.
- **`%` facts for both signs.** Unsigned values are the `b > 0`, `a >= 0`
  case; the signed ones cost three more implications and say what C's and
  Rust's `%` does.
- **The map keeps its mask.** With power-of-two bucket counts, `h & (n -
  1)` is one instruction where `%` is a division, once T is compiled; but
  it's right only for powers of two, which T can't yet say of `n`, so the
  map says so in a comment.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| let chains, `if let p = e && c` | the `else` is reached from several places: a labelled block, or the `else` written twice | — |
| a type on `let .. else` | the pattern's type arguments say it; a type would need ascription on `e` | — |
| precise `else` arms for local enums and aliases | their variants aren't known where the rewrite is; `_` is sound | — |
| facts about `/` by a variable | not asked for yet; the same reasoning gives them | — |
| `n` is a power of two, as a refinement | needs `n & (n - 1) == 0` facts, or a measure | — |

## Exercises

1. Write `let Option::Some(x) = o else { return 0 };` out by hand as lemma
   L2 says, in a function with statements after it, and check it does the
   same. Where do the statements go?
2. Make the `else` arm `_` in `Rewrite::else_pattern`. Which test in
   `tests/t` fails, and why is the program it checks sound?
3. `if let Option::Some(Option::Some(x)) = o` with `o: Option<Option<Fd>>`:
   what is `q`, and what does it throw away? Is that an error?
4. Why does a `let .. else`'s `else` need to be a `!` and not just
   diverge? Find a block that diverges but isn't a `!`, and see what the
   checker says.
5. Prove lemma L4 for `b < 0` and `a <= 0` from `r = a - b·trunc(a / b)`.
   Then: is `|a % b| <= |a|` needed to prove anything `|a % b| < |b|`
   doesn't?

## Reading

- Rust RFC 160, *if let* (2014), RFC 214, *while let* (2014), and RFC 3137,
  *let-else* (2021), whose rules T's follow; RFC 2497, *if-let chains*
  (2018), for what `&&` is kept for.
- Luc Maranget, *Warnings for pattern matching* (2007): usefulness, which
  says when the made-up arm is unreachable, and the missing patterns a
  complement arm is made of.
- Raymond T. Boute, *The Euclidean definition of the functions div and
  mod* (1992), and Daan Leijen, *Division and Modulus for Computer
  Scientists* (2001), on truncated, floored and Euclidean remainders.
