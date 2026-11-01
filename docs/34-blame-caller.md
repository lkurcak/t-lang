# Step 34: blaming the caller

Code: `parser.rs` (`Head::blame_caller`, `NOT_BLAMING`), `check.rs`
(`Checker::blaming_known`), `interp.rs` (`Blame`, `Interp::blamed`,
`Interp::made_at`), `cgen.rs` (`Gen::site`, `Gen::fn_value`, the `site_`
parameter), and `#[blame_caller]` throughout `core`, `alloc` and `std`.

When `unwrap` finds `None`, the panic happens in core, in `expect`'s body.
Showing that line would show nothing useful: the bug is the program's,
where it called `unwrap`. Since step 31, a panic inside core, alloc or std
has been shown at the innermost call outside them. That worked, but it was
a privilege of the built-in libraries. The hash map library couldn't have
it for its own checks, and neither could a test's helper. The C backend
paid for it with a global, `t_site`, set and restored around every call
from the program's own code.

Now any function can say it:

```rust
#[blame_caller]
fn check_even(n: u8) with panic {
    if n % 2 != 0 { panic("odd") }
}

fn main() {
    check_even(2);
    check_even(3); // shown here, not at the `panic`
}
// error: panicked: odd
```

core's `unwrap`, `expect`, `assert`, `assert_eq` and the rest say it, and
so do alloc's functions that allocate. Nothing about them is special any
more.

## What it means

An abort that happens *directly* in the body of a `#[blame_caller]`
function is shown where that function was called (B2). Directly means:

- one of its own operators: an overflowing `+`, a division by zero, an
  index out of bounds, an `as` that doesn't fit;
- a call of an intrinsic, like core's `panic` or alloc's buffer growth;
- a call of another function that blames its caller.

If that call is itself directly in the body of a function that blames its
caller, the abort moves out again, and so on. A function that doesn't
blame its caller ends the chain: an abort under its call is shown at the
call inside it, which is where the bug is if that function got its
arguments wrong.

```rust
#[blame_caller]
fn inner(n: u8) with panic {
    if n > 2 { panic("too big") }
}

fn middle(n: u8) with panic {
    inner(n + 1); // shown here
}

#[blame_caller]
fn outer(n: u8) with panic {
    middle(n);
}

fn main() {
    outer(2);
}
// error: panicked: too big
```

A closure's body is its own, whatever calls it. So a panic in the closure
given to `map` is shown in the closure, not at `map`.

"Abort" covers running out of memory too (D3). `List::push` blames its
caller, so a program that runs out of memory is shown at its own `push`,
not inside alloc's buffer code.

## Through a value

Static dispatch decides every call except one through a value: a `fn`
pointer, like `Option::unwrap` given to `map`. The call through it is
inside core's `map`, which has nothing to do with the bug. So a function
that blames its caller, used as a value, is blamed where it *became* a
value (B3): the `Option::unwrap` written in the program.

```rust
fn main() {
    let x: Option<Option<u8>> = Option::Some(Option::None);
    let _ = x.map(Option::unwrap); // shown at `Option::unwrap`
}
// error: panicked: `unwrap` of `None`
```

Blame stops there, even if what calls through the value blames its own
caller: the site belongs to the value, not to the call. An intrinsic used
as a value is treated the same way.

## Where it can be

On a function with a body (B1): a module's function, a function inside
another, an impl's method, or a trait's provided method. Not:

- on an `#[intrinsic]`: an intrinsic's aborts are already shown at its
  call, as an operator's are;
- on a trait's method without a body: it says something about a body, not
  about an interface or a type, so each impl says it of its own;
- on `main` or a `#[test]`: nothing in the program calls them;
- on anything that isn't a function.

It's an error on a function that can't abort (B4): one whose effects,
written or inferred, have no `panic`, no `alloc` and no effect variable.
An effect variable counts, because what it stands for may be a call of a
function that blames its caller. `fmt`'s `W::e` can be `String`'s
`alloc`.

```rust
#[blame_caller]
fn id(n: u8) -> u8 { n }

fn main() {
    let _ = id(1);
}
// error: `#[blame_caller]` on a function that can't abort
```

## Compiled

A function that blames its caller gets one more parameter in C, after its
own: `uint32_t site_`, the site of its call. In its body, every abort site
and every call of another function that blames its caller uses `site_`.
Every other body uses its own constant sites. A function used as a value
gets a wrapper for each place it was made a value, which passes that
place's site:

```text
static uint8_t f_Option_u8_unwrap_at7(T_Option_u8 a0) { return f_Option_u8_unwrap(a0, 7); }
```

There's no global any more, so nothing to save or restore, and nothing
that two threads would share. A call costs one more argument, and only a
call of a function that blames its caller. The interpreter works out the
same location from its stack of calls (`Interp::blamed`), and
`tests/compiled.rs` checks the two agree on every program.

## Why it's sound

B6 says what the attribute changes: the location in an abort's message,
and nothing else. Types, effects, values, drops, what's printed before
the abort, and the exit status are all the same with or without it. So
there's no soundness argument to make about programs' meaning, only these
claims:

- **Defined.** The location is always a place in the program's text,
  decided by the attributes of the functions on the call stack and, for a
  value, by where the value was made. It never depends on timing,
  optimisation, or debug information.
- **Local** (B5). It doesn't depend on which package a function is in.
  Moving code between packages, or adding a dependency, never changes
  where an abort is shown. The built-in libraries' privilege is gone,
  which was the last rule where package boundaries changed behaviour (see
  docs/29).
- **One meaning in both backends.** The interpreter and C compute it the
  same way, and the differential tests compare locations as they compare
  output.

## Design decisions worth noticing

- **Opt-in, not a line at the built-in libraries.** Step 31 drew the line
  at core, alloc and std to spare library authors an attribute. But the
  line was a privilege no other library could have, and it took a global
  to compile. An attribute is one word, says the same thing for every
  library, and compiles to one argument.
- **`blame_caller`, not `track_caller`.** Rust's `#[track_caller]` is the
  same idea. The name here says what it does in T's own words (an
  imperative, like `derive` and `allow`). The behaviour also differs:
  out-of-memory is covered, a value is blamed where it was made, and
  there's no `Location::caller()` to ask.
- **Closures aren't transparent.** A closure passed to `map` is the
  program's own code, so a panic in it is the program's to see there.
- **The built-in libraries are annotated wholesale.** Every function in
  them that can abort says it, except those whose only abort is through a
  closure they're given (`map`, `and_then`, `free_with`): there, the
  closure's body ends the chain anyway. That reproduces step 31's output
  exactly.

## Deliberately missing (coming later)

- A way for a program to ask where it was called from, as Rust's
  `Location::caller()` does. Nothing needs it yet, and it would make the
  location part of a program's meaning.
- On a trait's method declaration, applying to every impl.
- Backtraces: the whole chain, not just where it ends.

## Exercises

1. Remove `#[blame_caller]` from core's `expect` but keep it on `unwrap`.
   Where is a failed `unwrap` shown now, and why?
2. Write a function with an effect parameter whose attribute changes
   nothing however it's instantiated. Should B4 reject it? What would it
   need to know?
3. Give an example where blaming the place a value was made is wrong: the
   bug is at the call through the value.
4. With threads, why would the global `t_site` have been wrong, even if
   every thread had its own copy?

## Reading

- RFC 2091, *Implicit caller location*, and the Rust Reference on
  `#[track_caller]`: the same idea as a hidden parameter, and its rules
  for function pointers.
- Robert Bruce Findler and Matthias Felleisen, *Contracts for Higher-Order
  Functions* (2002): blame assignment, which party broke a contract, for
  functions passed as values.
