# Step 41: threads, `Mutex` and `unsafe impl`

Code: `std/thread.t` (`Threads`, `JoinHandle`, `spawn`, `join`),
`core/sync.t` (`Mutex<T>`), `Cell::as_mut` in `core/cell.t`,
`Checker::unsafe_impl` in `src/check/traits/collect.rs`, the threads of
the interpreter in `src/interp/threads.rs`, and the `std::thread` and
`spin` cases of `intrinsic_body` in `src/cgen.rs`, with
`t_thread_start` in `src/cgen/hosted.h`.

Since step 38, T knows which values may go to another thread (`Send`)
and which may be shared with one (`Sync`), and has atomics, which change
in steps no thread sees half of. This step adds the threads: a function
can start one, given the right to, and must wait for it to end. A thread
may borrow what its starter can reach, as it's over before that's gone.

```rust
use core::num::wrapping_add;

fn sum(xs: &[i64]) -> i64 {
    let mut s = 0i64;
    for x in xs.iter() {
        s = wrapping_add(s, *x);
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let data = [1i64, 2, 3, 4];
    let h = os.threads.spawn(|| sum(data.as_slice()));
    let mine = sum(data.as_slice());
    let theirs = h.join();
    let mut out = os.stdout;
    out.line(&wrapping_add(mine, theirs))?; // expect: 20
    Result::Ok(())
}
```

## Starting a thread

`Threads::spawn(&self, f)` starts a thread that runs closure `f`, and gives
back a `JoinHandle<'a, R>`, `R` being what `f` gives back. `join(self)`
waits for the thread to end, and gives back the `R`:

```text
export fn spawn<'a, R: Send + 'a, effect e, F: FnOnce() -> R with e + Send + 'a>(
    &self,
    f: F,
) -> JoinHandle<'a, R> with {alloc, e};

export fn join(self) -> R;
```

Each part of that says something:

- **`&self`, a `Threads`.** Starting threads is something a function can
  do only if it's handed the right to, like writing to a stream
  (docs/19-std-io.md): `main` gets the only `Threads` there is, in
  `os.threads`, and passes on a `&Threads` to what needs it. `Threads` is
  `Sync`, so a thread can be given one too, and start threads of its own.
- **`F: Send`, `R: Send`.** The closure goes to the new thread, and what it
  gives back comes back from it: both are handed from one thread to
  another. A closure is `Send` if what it captures is, so one that
  borrows `x` needs `x: Sync`, and one that moves `x` in needs `x: Send`.
- **`F: 'a`, `R: 'a`, `JoinHandle<'a, R>`.** Whatever the closure borrows
  lives at least as long as `'a`, and the handle no longer. So while the
  handle lives, the borrows do: nothing the thread reads can be changed,
  moved, or dropped until it's joined.
- **No `Discard` for `JoinHandle`.** A handle must be used up, and only
  `join` uses it up. That's what makes borrowing sound (lemma Z1 below).
- **`with {alloc, e}`.** What `f` may do, `spawn` does, so a function that
  says it can't panic can't start a thread that may. Starting a thread
  allocates its stack: if the operating system can't start one, the
  program stops, as when the memory runs out (docs/16-out-of-memory.md).
- **`join` gives back an `R`, not a `Result`.** A panic in any thread stops
  the whole program, as a panic always has: nothing unwinds, so there's
  no thread that ended by panicking to report.

What doesn't compile:

```rust,error
fn main(os: std::Os) with alloc {
    let mut n = 1u32;
    let h = os.threads.spawn(|| n);
    n = 5; // error: cannot assign to `n` while `n` is borrowed
    let _ = h.join();
}
```

```rust,error
// error: `h` is never used up, and its type, `JoinHandle<u32>`, isn't `Discard`
fn main(os: std::Os) with alloc {
    let data = [1u32, 2];
    let h = os.threads.spawn(|| data[0]); // warning: unused variable `h`
}
```

```rust,error
use core::cell::Cell;

// error: `Threads::spawn` needs its `F` to implement `Send`
fn main(os: std::Os) with alloc {
    let c = Cell::new(1u32);
    let h = os.threads.spawn(|| c.set(2));
    h.join();
}
```

## `Mutex`

Atomics share an integer. To share anything else, and change it, a
thread must have it to itself while it does: `core::sync::Mutex<T>`
lends what's in it to one thread at a time.

```rust
use core::sync::Mutex;
use std::thread::{JoinHandle, Threads};

#[derive(Send)]
struct Tally { sum: u64, adds: u32 }

fn add<'a>(threads: &Threads, t: &'a Mutex<Tally>, by: u64) -> JoinHandle<'a, ()> with alloc {
    threads.spawn(move || {
        let mut i = 0u32;
        while i < 10 {
            t.lock(|t: &mut Tally| {
                t.sum = core::num::wrapping_add(t.sum, by);
                t.adds = core::num::wrapping_add(t.adds, 1);
            });
            i = core::num::wrapping_add(i, 1);
        }
    })
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let t = Mutex::new(Tally { sum: 0, adds: 0 });
    let a = add(&os.threads, &t, 1);
    let b = add(&os.threads, &t, 100);
    a.join();
    b.join();
    let Tally { sum, adds } = t.into_inner();
    let mut out = os.stdout;
    out.line(&sum)?;  // expect: 1010
    out.line(&adds)?; // expect: 20
    Result::Ok(())
}
```

`lock(&self, f)` waits until no other thread has the mutex, then calls `f`
with a `&mut T` to what's in it, and gives back what `f` does. The
reference can't get out of the closure: `f` must take one of any
lifetime, which its result can't name (docs/06e.1-higher-ranked-closures.md).
So the lock is held exactly while `f` runs, and there's no guard to
forget to drop. A mutex not shared needs no locking: `get_mut(&mut self)`
and `into_inner(self)` reach what's in it through the `&mut` or by
value.

There's no *poisoning*: Rust marks a mutex whose holder panicked, as
what's in it may be half changed. In T a panic stops the program, so no
one ever sees that.

A mutex is plain T, in `core::sync`, so it works without an operating
system, and in a static (`Mutex::new` is worked out while compiling):

```text
static LOG: Mutex<[u32; 4]> = Mutex::new([0, 0, 0, 0]);
```

It's two fields: an `Atomic<bool>`, set while some thread has the lock,
and a `Cell<T>`, what's in it. `lock` swaps `true` in until it swaps out
`false`, telling the machine meanwhile that it's waiting (on the hosted
target, letting another thread run), and stores `false` when `f` is done.
Locking it again inside `f` waits for ever: the closure already has
what's in it.

## `unsafe impl`

By its fields, a mutex isn't `Sync`: a `Cell<T>` isn't, as two threads
could change it through two `&`s. What the fields don't show is the lock:
only the thread that has it reaches the cell. So the mutex is `Sync`
anyway, if `T` may go from thread to thread, which is what happens to it:

```text
unsafe impl<T: Send> Sync for Mutex<T> {}
```

An `unsafe impl` is its writer's promise, which the checker can't check:
like an `unsafe` block (docs/09-unsafe.md), it's where to look if it's
wrong. It's allowed only where the promise can be kept, and only so far
as nothing else changes:

- only of `Send` or `Sync`, which have no methods, so the impl is empty;
- only in the package that declares the type, which alone sees its
  private fields, and keeps its promise as they change (the same rule as a
  written derive bound, docs/24-visibility.md);
- only for the type with any parameters, `Mutex<T>`, as a derive is: one
  impl per type and trait, never beside a derive of the same trait;
- needing only `Send` or `Sync` of the parameters, so a proof of `Send` or
  `Sync` still needs nothing but the two (lemma D1 of
  docs/17-retrospective-1.md). A derived `Sync` can now need `Send` of a
  parameter, as a struct with a `Mutex<T>` field does.

Without `unsafe`, `impl Sync for ..` is still refused: those are derived.

What a lock does with its cell needs one more thing: a `&mut` to what's in
the cell, from a `&` to it. That's `Cell::as_mut(&self) -> &mut T`, which
is `unsafe`: its caller must make sure nothing else reaches what's in the
cell while the reference lives. With it, a library can write a lock of its
own, as `Mutex` is written:

```rust
use core::cell::Cell;
use core::sync::Atomic;

#[derive(Discard where T: Discard, Send where T: Send)]
struct Flagged<T> { busy: Atomic<bool>, value: Cell<T> }

unsafe impl<T: Send> Sync for Flagged<T> {}

impl<T> Flagged<T> {
    fn new(value: T) -> Flagged<T> { Flagged { busy: Atomic::new(false), value: Cell::new(value) } }

    // Runs `f` if no other thread is, or gives back `false`.
    fn try_with<F: FnOnce(&mut T)>(&self, f: F) -> bool where F: Discard {
        match self.busy.swap(true) {
            true => false,
            false => {
                // SAFETY: `busy` was `false`, and is `true` until `f` is done:
                // no other `as_mut` of the cell meanwhile, and nothing else
                // reaches it.
                f(unsafe { self.value.as_mut() });
                self.busy.store(false);
                true
            },
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let f = Flagged::new(1u64);
    let ran = f.try_with(|x: &mut u64| *x = core::num::wrapping_add(*x, 41));
    let mut out = os.stdout;
    out.line(&ran)?;           // expect: true
    out.line(&f.value.get())?; // expect: 42
    Result::Ok(())
}
```

## How they're run

Compiled, a thread is a POSIX thread (`pthread_create`, with an 8 MiB
stack), and `join` is `pthread_join`. The closure, what runs it, and room
for its result are in a *packet* on the heap, which `join` frees. When a
thread panics, it alone writes its message: any other thread that gets
to a panic meanwhile waits for the program to end.

The interpreter (`src/interp/threads.rs`) runs each thread on a thread of
its own, but one at a time: a thread runs until it does something
another thread could see, an atomic operation, a mutex's wait, or a
`join` of a thread that hasn't ended, and then the next thread that can
run has a turn, in the order they were started, round and round. So every
run is the same, and a thread that waits for another, spinning on an
atomic, lets it run. Between those points a thread runs alone; for a
program the checker accepted, that's no loss, as it has no data races
(Z2), so nothing it does between them could be seen half done anyway. If
every thread waits to join another, the interpreter stops with
*deadlock*; a compiled program would wait for ever.

## Why it's sound

> A thread reaches only what it was handed (`Send`) or shares (`Sync`),
> for no longer than its handle lives; and what's shared changes only
> atomically, or by one thread at a time.

- **Z1 (scoped threads).** A `JoinHandle<'a, R>` keeps the closure's
  borrows alive (`F: 'a`) until it's used up, and only `join` uses it up,
  which returns once the thread has ended. It can't be dropped (no
  `Discard`), and nothing in T loses a value without using it up: there's
  no `forget`, no reference counting to make a cycle of, no unwinding,
  and a value whose type isn't `Discard` makes whatever holds it, a box,
  a list, an arena, a mutex, not `Discard` either; a static's type must be
  `Discard`. So on every path, either the handle is joined before `'a`
  ends, or the path never ends (it loops, or the program stops), and the
  borrowed values are never gone while the thread runs. This is Rust's
  first scoped-thread API, `thread::scoped`, which was withdrawn before
  Rust 1.0 because `mem::forget` and `Rc` cycles leak a guard safely:
  T has neither.
- **Z2 (no data races).** A thread gets only `Send` values (the closure)
  and gives back only `Send` values (the result); a `&` it holds is to a
  `Sync` value. By Y1 of docs/38-statics.md, the only things written
  through a `&` are cells, arenas and atomics; a `Sync` value's cells are
  inside a type whose `unsafe impl Sync` promises they're reached by one
  thread at a time. `Mutex` keeps that promise: `locked` is `true` from a
  successful `swap(true)` until `store(false)`, so at most one thread is
  between them, and only there is `as_mut` called; the `&mut` it gives
  is the closure's for that stretch only (it can't escape, as the
  closure's parameter is of any lifetime). The atomics are sequentially
  consistent, so what one holder wrote, the next sees.
- **Z3 (what's in a mutex is opaque).** Its fields are private to
  `core::sync`, and in a `Cell`, so no measure, refinement or path fact
  reads them (K1 of docs/37-cell.md), and the mutex is invariant in `T`
  and not frozen, as a struct with a cell in it is: no const can hold one,
  and `&Mutex<&'static str>` can't be used as `&Mutex<&'a str>` to put a
  shorter reference in.
- **Z4 (effects).** `spawn` has the effects of `f`: a thread's panics,
  allocations and statics are its starter's, so `with {}` still means a
  call does none of them, on any thread (H7 of
  docs/17-retrospective-1.md: running for ever is still no effect's).
  The interpreter checks it: a new thread starts with its starter's
  stack of effects to check against.
- **Z5 (`unsafe impl` is local).** It's allowed only where the type is
  declared, for the type with any parameters, with bounds of `Send` and
  `Sync` only: so whether a type is `Send` or `Sync` is still decided by
  its own package, and adding a package changes nothing about it
  (docs/29-closed-and-sealed-traits.md). Proofs of the two still need
  only the two, so the derive's least fixed point still ends.

## Design decisions worth noticing

- **Handles that must be joined, not a scope.** Rust's `thread::scope(|s|
  ..)` joins every thread at the end of a closure, as Rust can't stop a
  handle being forgotten. T can, so a handle is an ordinary value, joined
  where the code says, returned from functions, or handed to another
  thread to join.
- **The right to start threads is a value.** Like the streams, it's in
  the `Os` that `main` is given, rather than an effect anyone may have:
  a library that wasn't handed a `Threads` can't start one.
- **No failure to start.** Rust's `Builder::spawn` gives a `Result`, which
  here would have to give back the closure, as it may hold values that
  must be used up. A thread is a stack, and running out of memory already
  stops the program.
- **A closure, not a guard.** `lock(|x| ..)` holds the lock exactly while
  the closure runs, with no guard type, no `Deref`, and no unlocking in a
  destructor. A guard can come later, if code needs locks held across
  calls in ways a closure can't say.
- **`Mutex` spins.** It waits by trying again, letting other threads run
  meanwhile, rather than asking the operating system to wake it, which
  is faster when it waits long. It's in `core`, so it works on the
  freestanding target, in statics, and in interrupt-free kernels; a mutex
  that sleeps can come to std later, with the same interface.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| channels, `Condvar`, `RwLock`, `Once` | built on `Mutex` and atomics, in libraries | — |
| a mutex that sleeps instead of spinning | needs the operating system's wait and wake | — |
| a lock guard | the closure covers what's needed so far | — |
| thread-local statics | no need yet | — |
| threads on the freestanding target | a platform would have to provide them | — |
| weaker atomic orderings | SeqCst is the safe default | — |

## Exercises

1. Remove `F: 'a` from `spawn` and write a program that reads a value
   after it's gone. Which check would catch it if the handle were
   `Discard`?
2. Make `Mutex<T>: Sync` need `T: Sync` instead of `T: Send`. Is it still
   sound? What can't you put in one then?
3. Write a `Once<T>` in T: a value worked out by the first thread to ask
   for it. Which `unsafe impl` does it need?
4. Why may a `JoinHandle<'a, R>` be handed to another thread to join? What
   must `R` be?
5. Write a program that deadlocks two threads with two mutexes. What does
   the interpreter do? A compiled one?

## Reading

- The Rust standard library's `std::thread::scope` and `std::sync::Mutex`
  docs, and the Rustonomicon's *Send and Sync*.
- Aaron Turon, *The Scoped Threads Saga* (and Rust issue #24292,
  "`thread::scoped` is unsound"): why Rust's first API went.
- Maurice Herlihy and Nir Shavit, *The Art of Multiprocessor
  Programming* (2008): spin locks, and what sequential consistency gives
  them.
- Ralf Jung et al., *RustBelt: Securing the Foundations of the Rust
  Programming Language* (2018): `Send`, `Sync` and `unsafe impl` as
  semantic promises about a type's invariant.
