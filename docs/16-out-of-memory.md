# Step 16: out of memory is `alloc`

Code: `alloc/raw.t` (`out_of_memory`, `RawBuf::grow`), `alloc/list.t`
(`List::push`), `check/paths.rs` (`Checker::box_new`), and
`Interp::out_of_memory` in `src/interp.rs`.

Since step 7, a function that allocates says `with alloc`, and one that
may abort says `with panic`. Running out of memory aborts, so until now
everything that allocated said both, and `panic` stopped meaning anything
in particular: in tcalc, every function that pushed to a `String` said
`with {alloc, panic}`, whether or not it had a bug to abort on. Now
running out of memory is part of `alloc`, and `panic` means a bug:

```rust
use alloc::list::List;
use alloc::string::String;

fn upto(n: i64) -> List<i64> with alloc {
    let mut l = List::new();
    for i in 0..n {
        l.push(i);
    }
    l
}

fn greet(name: &str) -> String with alloc {
    let mut s = String::from("hello, ");
    s.push_str(name);
    s.push('!');
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l = upto(4);
    let last = *l.get(3).unwrap_or(&0);
    l.free();
    out.line(&last)?; // expect: 3
    let s = greet("world");
    let written = out.line(&s.as_str()); // expect: hello, world!
    s.free();
    written?;
    Result::Ok(())
}
```

`upto` can't panic: counting through a range can't overflow
(docs/08-refinement-types.md), and `push` only allocates. Neither can
`main`.

## What each effect promises

> **Lemma M1.** A function whose effects don't have `panic` never reaches
> a bug's abort: an operator's, `panic`'s, `unwrap`'s. One whose effects
> don't have `alloc` never allocates, and never runs out of memory. One
> with neither never aborts, but for running out of stack.

The proof is docs/07-effects.md's, by induction on the stack, once for
each effect. What's new is only which effect each abort is: running out
of memory happens at `Box::new`, `RawBuf::new`, `RawBuf::grow` and
`alloc::raw::out_of_memory`, and each of them says `alloc`. The
interpreter checks it at run time: where memory runs out, every function
on the stack must say `alloc`, or it stops with "type soundness
violated". The third promise is what docs/07 promised of a function
without `panic`; it now takes `alloc` missing too, and that's the price
of the first: "can't panic" no longer says "can't abort".

```rust,error
use alloc::raw::out_of_memory;

fn give_up() -> ! with panic { out_of_memory() }

fn main() with panic {
    give_up();
}
// error: `out_of_memory` may allocate, but `give_up` doesn't say `with alloc`
```

## The alloc library without `panic`

For `alloc` to say only `alloc`, its code must not panic for other
reasons. Two places did:

- **`RawBuf::grow(new_cap)` panicked when `new_cap` was less than
  `cap()`**, as shrinking could lose values. That's a bug in its caller,
  so a panic. Now it's `grow(extra)`, which adds `extra` slots: it can't
  shrink, so it has nothing to check. More slots than a `usize` can count
  are more memory than there is, so `cap() + extra` overflowing is
  running out of memory, as asking for that many would be.
- **`List::push` doubled its buffer, `grow(cap * 2)`, and then added one
  to `len`.** The first is now `grow(cap)`, with nothing to overflow. The
  second can't overflow either, as `len < cap()` after growing, but the
  checker can't see that: a fact can't say `cap()` (measures are for
  later). So `push` says it a way the checker can see:

```text
if self.len == greatest() {
    out_of_memory();
}
```

A list as long as a `usize` can count can't count one more value, which
is running out of memory too. After the `if`, `len` isn't the greatest
`usize`, so `len + 1` is proved to fit. It's never true, but for a list of
values that take no memory, and it's one comparison.

`out_of_memory()` is public, for the same reason: a collection of a
program's own needs it where `push` does. As Rust's `handle_alloc_error`,
it aborts the program, `-> ! with alloc`. Calling it where memory hasn't
run out breaks no promise, since every function that says `alloc` may
abort anyway; it's just wrong.

```rust
use alloc::raw::RawBuf;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut b: RawBuf<i64> = RawBuf::new(2);
    b.grow(3);
    let five = b.cap() == 5;
    // SAFETY: nothing was written.
    unsafe { b.free() };
    out.line(&five) // expect: true
}
```

## Design decisions worth noticing

- **One effect, not two.** Anything that allocates may run out, so an
  `oom` effect would always come with `alloc`, and say nothing on its
  own. When allocating can fail without aborting, `Box::try_new`, that
  may change: it allocates, and doesn't abort.
- **Asking for more than can be counted is running out.** `grow`'s
  overflow and `push`'s counter are the same as an allocator saying no:
  the program can't have the memory it asked for. Rust's `Vec` panics on
  "capacity overflow" as a different message; T's is one abort, under
  `alloc`.
- **The library is checked without `panic`**, so its signatures are
  proofs, not promises: `List::push` says `with alloc` because the checker
  showed nothing in it panics.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| fallible allocation, `Box::try_new`, `List::try_push` | needs an allocator design, and a `Result` for each | — |
| `List` proving `len < cap()` itself | needs measures, `cap()` in a fact | — |
| catching running out of memory | would need effect handlers, as catching a panic would | — |

## Exercises

1. Take the `out_of_memory()` guard out of `List::push`. What does the
   checker say, and which fact is it missing?
2. Make `RawBuf::new` say `with panic` instead of `with alloc` in
   `alloc/raw.t`. What does the checker say about `alloc/list.t`? Then
   make it say no effects at all, and write a program that the checker
   accepts and the interpreter stops. Which promise of lemma M1 does it
   break?
3. Write `fn try_push(l: &mut List<i64>, v: i64) -> bool with alloc` that
   pushes only while the list is shorter than 1000. Can it be written so
   that it can't panic? So that it can't abort?
4. Rust's `Vec::with_capacity` panics on "capacity overflow" and aborts on
   an allocation error. Argue that these are, or aren't, the same kind of
   failure, from what a caller can do about each.

## Reading

- Rust's `std::alloc::handle_alloc_error` and the RFC 2116 discussion,
  *fallible collection allocation* (2017), on aborting versus returning an
  error when memory runs out.
- The Linux kernel's rules on allocation, in its coding style document:
  allocation that may fail, and code that must check it, as the default.
- Daan Leijen, *Koka: Programming with Row Polymorphic Effect Types*
  (2014), on effects that say what a function may do, kept apart by kind.
