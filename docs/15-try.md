# Step 15: `?`

Code: `core/ops.t` (`Try`, `ControlFlow`), `check/modules.rs`
(`Rewrite::try_expr`), and the messages in `check/paths.rs` and
`check/traits/prove.rs`.

Code that can fail returns a `Result` or an `Option`, and code that calls
it mostly passes a failure on to its own caller. Until now, each such call
was a `match`:

```rust
fn twice(x: Result<i64, bool>) -> Result<i64, bool> with panic {
    let v = match x {
        Result::Ok(v) => v,
        Result::Err(e) => return Result::Err(e),
    };
    Result::Ok(v * 2)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&twice(Result::Ok(4)).unwrap_or(0))?; // expect: 8
    Result::Ok(())
}
```

A third of the parser in tcalc, the first program written in T outside
this repository (<https://github.com/lkurcak/tcalc>), was this. `e?` says
it in one character:

```rust
fn half(x: i64) -> Result<i64, bool> {
    if x % 2 == 0 { Result::Ok(x / 2) } else { Result::Err(x > 0) }
}

fn quarter(x: i64) -> Result<i64, bool> {
    let h = half(x)?;
    half(h)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&quarter(8).unwrap_or(0))?; // expect: 2
    out.line(&quarter(6).unwrap_or(0))?; // expect: 0
    Result::Ok(())
}
```

## A `?` is a `match`

docs/05.1-control-flow.md left `?` for later because it "needs a way to
say what can be returned early, a trait". That's `core::ops::Try`:

```text
pub enum ControlFlow<B, C> { Continue(C), Break(B) }

pub trait Try {
    type Output;
    type Residual;
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
    fn from_residual(r: Self::Residual) -> Self;
}
```

`branch` says whether to go on, with the `Output`, or to stop, with the
`Residual`: what's left of the value when there's nothing to go on with.
`from_residual` makes a value of the function's own result type from it.

**Lemma Q1 (a `?` is a `match`).** `e?` means

```text
match core::ops::Try::branch(e) {
    core::ops::ControlFlow::Continue(v) => v,
    core::ops::ControlFlow::Break(r) => return core::ops::Try::from_residual(r),
}
```

with `v` and `r` names no program can write. Resolving paths rewrites it
into just that, naming the library's items by their symbols, as it does a
`for` (lemma C4, docs/06e-core.md), so a program's own `Try` or
`ControlFlow` doesn't change it. Nothing after that knows `?` exists.
It has no typing rule, borrow rule, linearity rule, effect rule or
refinement rule of its own: what holds for a `match`, a `return` and two
calls holds for it. What follows from that:

- `e`'s type implements `Try`, and so does the type the function returns,
  as `from_residual`'s `Self` is what the `return` gives.
- Their residuals are the same type, as `r` goes from one to the other.
- It has no effects of its own: `Try`'s methods have none, and an impl's
  method can't have more effects than its trait's says (docs/07-effects.md).
- In a closure, it returns from the closure, as `return` does.
- A value that must be used up, and is still there at a `?`, must be
  used up on the way out too, as at a `return`.

The rewrite's names are at an empty span at the `?`, which is where an
error about them is. The checker knows those spans (`Checker::is_try`),
and says what's wrong in terms of the `?` rather than of `branch` and
`from_residual`:

```rust,error
fn get() -> Option<i64> { Option::None }

fn main() {
    let x = get()?;
    let _v = x;
}
// error: `?` returns early from `main`, but its result, `()`, doesn't implement `core::ops::Try`
```

## Residuals keep kinds apart

**Lemma Q2 (residuals).** An `Option<T>`'s residual is `Option<!>`, and a
`Result<T, E>`'s is `Result<!, E>`. `!` has no values (docs/05.1), so an
`Option<!>` can only be `None`, and a `Result<!, E>` only an `Err`. That
makes `from_residual` total with a `match` of one arm, which the
exhaustiveness checker accepts as it is. And as the residuals must be
equal, a `?` stops only into a type that can say the same failure:

- an `Option`'s `?` returns `None` from a function returning any
  `Option`, and from nothing else;
- a `Result`'s `?` returns its `Err` from a function returning a `Result`
  with the same error type.

The residual is a type of its own, and not just what's in the failure,
on purpose. Were a `Result<T, E>`'s residual `E` and an `Option`'s `()`,
a `None` would quietly turn into an `Err(())`:

```rust,error
fn get() -> Option<i64> { Option::None }

fn f() -> Result<i64, ()> {
    let x = get()?;
    Result::Ok(x)
}

fn main() {
    let _ = f();
}
// error: `?` can't return early here: `f` returns `Result<i64, ()>`, so a `?` in it must stop with `Result<!, ()>`, and this one stops with `Option<!>`
```

An error isn't converted: a `Result<T, i32>`'s `?` doesn't go into a
function returning `Result<U, bool>`. Rust converts it with `From`, by
making the residual a parameter of a second trait, `FromResidual<R>`.
T's traits have no type parameters yet, so that waits for them. Until
then, `.map_err(..)?` says the conversion where it happens.

## Linear values

A `?` may return, and what's in scope then goes out of scope. If that's a
value that must be used up, that's an error, as it would be at a
`return`, and the note says it's the `?`'s doing:

```rust,error
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn get() -> Option<i64> { Option::None }

fn f() -> Option<i64> with panic {
    let fd = Fd { n: 1 };
    let x = get()?;
    Option::Some(x + close(fd))
}

fn main() with panic {
    let _ = f();
}
// error: `fd` goes out of scope here, if this `?` returns early
```

Close it first, or call what can fail before making it. What's *in* a
`Result` can be linear, both the `Ok`'s value and the `Err`'s: `?` moves
each of them once, to where it goes.

## A program's own `Try`

`Try` is an ordinary trait, so a program can implement it for its own
types, with the orphan rules as usual. That can't make a `?` unsound, as
what it means is calls of those methods, checked like any others; at
worst a wrong impl gives wrong answers. Here, a parse that stops at its
first error, with where it was as what goes up:

```rust
use core::ops::{ControlFlow, Try};

#[derive(Discard)]
enum Parsed<T> { Done(T), Failed(usize) }

#[derive(Discard)]
struct At { at: usize }

impl<T> Try for Parsed<T> {
    type Output = T;
    type Residual = At;

    fn branch(self) -> ControlFlow<At, T> {
        match self {
            Parsed::Done(v) => ControlFlow::Continue(v),
            Parsed::Failed(at) => ControlFlow::Break(At { at }),
        }
    }

    fn from_residual(r: At) -> Parsed<T> { Parsed::Failed(r.at) }
}

fn digit(s: &str, at: usize) -> Parsed<u32> {
    match s.as_bytes().get(at) {
        Option::Some(b) => match *b {
            0x30..=0x39 => Parsed::Done(*b as u32 - 0x30),
            _ => Parsed::Failed(at),
        },
        Option::None => Parsed::Failed(at),
    }
}

fn two(s: &str) -> Parsed<u32> with panic {
    let a = digit(s, 0)?;
    let b = digit(s, 1)?;
    Parsed::Done(a * 10 + b)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    match two("4x") {
        Parsed::Done(v) => out.line(&v)?,
        Parsed::Failed(at) => out.line(&at)?, // expect: 1
    }
    Result::Ok(())
}
```

## Design decisions worth noticing

- **A trait and a rewrite, not a rule.** A built-in rule for `Result` and
  `Option` would be less library, but a new rule in every pass (types,
  borrows, linearity, effects, refinements, the interpreter), each with
  its own argument. The rewrite can't disagree with the rules it's made
  of.
- **`ControlFlow`, not `Result`, for `branch`.** Stopping early isn't
  always an error: for a program's own type it may be finding what it
  looked for. A type that says "go on" or "stop" says what it means.
- **Residuals, not what's in the failure.** See lemma Q2: they keep an
  `Option` from becoming a `Result`.
- **No conversion.** It needs traits with type parameters, and it's a
  relaxation: a program that checks now will check then.
- **`?` binds as tightly as a call.** `-x?` is `-(x?)`, and `a?.b()?` is
  `(a?.b())?`, as in Rust.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| converting the error, `From` | needs traits with type parameters | — |
| `let p = e else { .. };`, `if let` | each needs rules of its own, as there are no tuples to rewrite them with; `?` covers most of what they'd be for | — |
| `try { .. }` blocks | a `?` in one would stop the block, not the function: a labelled block to `break` out of, which T doesn't have (docs/05.1) | — |
| `?` in `main` | `main` returns `()`: it would need an exit status, so an operating system | — |

## Exercises

1. Write `?` out by hand, as lemma Q1 says, in a function with a `for`
   loop in it, and check the program does the same. Which `return` does
   your `match` have, and which `break` does the `for` have?
2. Make `Option`'s residual `()` and `Result`'s `E`, in `core/ops.t`.
   Which test in `tests/t` fails, and why does it check without the
   change in Q2?
3. Implement `Try` for a type whose residual is `Option<!>`. Where can its
   `?` return early, and is that what you'd want?
4. Why can't `Try::branch` have an effect parameter, the way `Iterator`
   has `effect e`? What would a `?` then have to say about its effects?
5. With `From`, `?` would return `Err(From::from(e))`. Where would the
   conversion's effects go, and what would that say about a function that
   uses `?` and can't panic?

## Reading

- Rust RFC 243, *Trait-based exception handling* (2014), which proposed
  `?`, and RFC 3058, *try_trait_v2* (2020), whose `Try`, `ControlFlow` and
  residuals are what T has, less `FromResidual`'s parameter.
- Philip Wadler, *Comprehending Monads* (1990): `?` is a monad's bind,
  written inside out, for the one monad a function's result type is.
- Joe Duffy, *The Error Model* (2016), on errors as values whose passing
  on is written, but short.
