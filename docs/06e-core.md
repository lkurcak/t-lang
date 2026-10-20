# Step 6e: a small core library

Code: `core/` (the library itself, in T), `src/load.rs` (reading it into
every program), `src/check/modules.rs` (`core::` paths, the prelude, and
what `for` and `a..b` mean), `src/check/traits.rs` (`where` clauses and the
orphan rule), and `src/interp.rs` (the one intrinsic).

Every program has the core library. What its prelude brings in needs no
`use`:

```rust
#[derive(Discard)]
struct Countdown { n: int }

impl Iterator for Countdown {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.n == 0 {
            return Option::None;
        }
        self.n = self.n - 1;
        Option::Some(self.n + 1)
    }
}

fn main() {
    for i in (Countdown { n: 3 }) {
        print(i);
    }
    // expect: 3
    // expect: 2
    // expect: 1
    let total = (0..4).map(|x| x * x).fold(0, |a, b| a + b);
    print(total);    // expect: 14
    print(5.max(9)); // expect: 9
}
```

The library is T code, checked by the same rules as any program's, and it
uses nothing a program couldn't. It needs no allocation and no I/O, so it
can be anywhere T can, down to bare metal.

## What's in it

| Module | What | In the prelude |
|---|---|---|
| `core::option` | `Option<T>`: `is_some`, `is_none`, `as_ref`, `as_mut`, `take`, `map`, `and_then`, `or_else`, `unwrap_or`, `unwrap_or_else` | `Option` |
| `core::result` | `Result<T, E>`: `is_ok`, `is_err`, `as_ref`, `map`, `and_then`, `map_err`, `or_else`, `ok`, `err`, `unwrap_or`, `unwrap_or_else` | `Result` |
| `core::cmp` | `Ordering`, and `Ord`: `cmp`, `lt`, `le`, `gt`, `ge`, `max`, `min`, for `int` and `bool` | `Ord` |
| `core::iter` | `Iterator`: `next`, `map`, `filter` (6e.1), `fold`, `count`; `Map`, `Filter` | `Iterator` |
| `core::ops` | `Range<T>`, what `a..b` makes, an `Iterator` for `int`s | — |
| `core::mem` | `swap` and `replace` | — |

The traits built into the language, `Copy`, `Discard`, `Eq` and the `Fn`
traits, are everywhere as before. They aren't the library's: `==`,
`#[derive]` and closures are defined by what they are.

A method that would throw a value away says so with a bound, on the
method, not on the type, so an `Option` of something linear still has all
the rest. `unwrap_or` throws one of its two values away, and is only for
a `T` that's `Discard`:

```rust
struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let mut slot = Option::Some(Fd { n: 1 });
    print_bool(slot.is_some()); // expect: true
    match slot.take() {
        Option::Some(fd) => print(close(fd)), // expect: 1
        Option::None => {},
    }
    match slot {
        Option::Some(fd) => print(close(fd)),
        Option::None => print(0), // expect: 0
    }
}
```

`unwrap_or_else` makes the default only when it's needed, with a closure.
Then nothing is thrown away but the closure, uncalled, so it works for a
linear `T`, as long as what the closure captures may be thrown away.
`Result`'s gives the error to the closure, so `E` needn't be `Discard`
either.

There's no `unwrap`. It would panic on `None`, and until step 7 nothing in
a function's type says it can panic.

## The prelude comes last

The prelude is what `core::prelude` brings in: `Option`, `Result`, `Ord`
and `Iterator`. Every module sees it, after its own items and `use`s.

**Lemma C3 (the prelude can't change a program).** A name a module declares
or imports means that, whatever the prelude has. So which item a name means
is still one thing (lemma M1 of docs/06d-modules.md), and adding to the
prelude never changes what a program that already works means.

A program's own `Option` is its own, and `core`'s is still there by its
path:

```rust
#[derive(Discard)]
enum Option { Some(int), Empty }

fn mine(n: int) -> Option {
    if n > 0 { Option::Some(n) } else { Option::Empty }
}

fn theirs(n: int) -> core::option::Option<int> { core::option::Option::Some(n) }

fn main() {
    match mine(1) {
        Option::Some(n) => print(n), // expect: 1
        Option::Empty => {},
    }
    print(theirs(2).unwrap_or(0)); // expect: 2
}
```

There's one way the prelude can still make trouble: its traits are in
scope everywhere, for method calls. A trait added to it with a method
`size` would make `x.size()` ambiguous where a module's own trait has one
too. So the prelude has few traits, chosen on purpose.

`Ordering`, `swap`, `replace` and `Range` need a `use`, as they do in Rust.
They're written less often, and every name in the prelude is one more that
appears from nowhere.

## Two packages

The core library is a package of its own, a module tree beside the
program's. A path can start with `core` in any module, as it can with
`crate`. In the library, `crate` means its own root. No module of one
package is inside the other, so `super` can't leave a package, and its
private items are its own.

```rust,error
fn main() {
    let x: crate::core::option::Option<int> = Option::None;
}
// error: the crate root has no item `core`
```

No module of a program can be called `core`, so `core::a` always means the
library's `a`.

Each item of the library has a symbol that starts with `core::`, like
`core::option::Option`. Lifting, monomorphisation and the interpreter only
see symbols, so for them the library is just more items.

## Orphans

**Lemma C2 (coherence, package by package).** An impl of a trait for a
type must be in the package of the trait, or of the type's head: its
outermost constructor, `Option` in `Option<Mine>`. The built-in traits and
types, `int`, `&T`, `Box<T>` and the rest, are the core library's. So:

- the library's impls are all there are of its traits for its types,
  whatever program it's in; and
- no program can add one that overlaps one of them, or that another
  program might add differently.

Coherence (docs/06-traits.md) is then a property of each package checked
on its own: the library's impls never conflict with a program's, because
they can't be for the same trait and type.

```rust,error
use core::cmp::Ordering;

impl Ord for int {
    fn cmp(&self, other: &int) -> Ordering { Ordering::Equal }
}

fn main() {}
// error: `Ord` and `int` are both `core`'s, so only `core` can implement one for the other
```

Only the head counts, so `impl Iterator for Option<Mine>` is rejected too,
and a blanket `impl<T: Eq> Ord for T` most of all: it would cover `int`.
Rust lets a local type further in count, under rules that took several
RFCs to get right. That's a relaxation for later, if it's needed.

## Trusting one thing

**Lemma C1 (the library is checked).** Everything in the core library is
checked like any program, except what an *intrinsic* does: a function
declared with `#[intrinsic]` and no body, which the compiler provides.
There is exactly one, `core::mem::swap`:

```text
#[intrinsic]
pub fn swap<T>(a: &mut T, b: &mut T);
```

Its signature is checked like any other. What the checker takes on trust
is what it does. Two `&mut`s never alias (the borrow checker makes sure),
and exchanging what they point to makes no value and throws none away, so
it's sound for every `T`, linear ones too. `replace` is written in T with
it:

```text
pub fn replace<T>(dest: &mut T, mut src: T) -> T {
    swap(dest, &mut src);
    src
}
```

so nothing more is trusted. It can't be the other way round: to write
`swap` with `replace`, you'd need a `T` to put in first, and code that's
generic over `T` has none. Only the core library can declare an intrinsic.

## `for` loops

**Lemma C4 (a `for` is a `loop`).** `for p in it { body }` means

```text
{
    let mut iter = it;
    loop {
        match core::iter::Iterator::next(&mut iter) {
            core::option::Option::Some(p) => body,
            core::option::Option::None => break,
        }
    }
}
```

with the `for`'s label, if it has one, on the `loop`, and `iter` a name no
program can write. Resolving paths rewrites it into just that, naming the
library's items by their symbols, so a program's own `Option` or
`Iterator` doesn't change it. Nothing after that knows `for` exists, so it
needs no typing rule, borrow rule or linearity rule of its own: what holds
for the `loop` holds for it.

Two things follow. When the loop ends, `iter` goes out of scope, which is
fine only if its type is `Discard` (docs/06a.1-linear-types.md). Each value
`p` binds is the body's, to use up like any variable, on every path through
it, `break` and `continue` included.

```rust,error
struct Lines { left: int }

impl Iterator for Lines {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.left == 0 {
            return Option::None;
        }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn main() {
    for n in (Lines { left: 2 }) {
        print(n);
    }
}
// error: a `for` throws its iterator away when it ends, and `Lines` isn't `Discard`; to keep it, iterate over a mutable reference to it: `for x in &mut it`
```

An iterator that must be used up is iterated through a `&mut`, which may
always be thrown away. The library has `impl<I: Iterator> Iterator for &mut
I`, which calls `I`'s `next`. Afterwards it's still there, for its owner to
close, and after a `break` what it hasn't given yet is still in it:

```rust
struct Lines { left: int }

impl Iterator for Lines {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.left == 0 {
            return Option::None;
        }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn close(lines: Lines) -> int {
    let Lines { left } = lines;
    left
}

fn main() {
    let mut lines = Lines { left: 5 };
    for n in &mut lines {
        if n == 2 {
            break;
        }
        print(n);
    }
    // expect: 4
    // expect: 3
    print(close(lines)); // expect: 2
}
```

A `for` is `()`. It can end without a `break`, so a `break` can't give it a
value, as for `while`:

```rust,error
fn main() {
    let x = for i in 0..3 {
        break i;
    };
}
// error: a `break` with a value only works in a `loop`: a `for` loop can end without a `break`
```

A loop that hands back what it didn't use, or that ends with a value of its
own, would be a different protocol: `next` taking the iterator by value
and giving it back with each item, or a final value when it's done. That's
a direction for later, with effects.

## Ranges

`a..b` is `core::ops::Range::new(a, b)`: from `a` up to, and not including,
`b`. Its operands bind more tightly than it does, so `0..n + 1` is
`0..(n + 1)`.

**Lemma C5 (counting up stops).** `Range<int>`'s `next` checks that `start`
is before `end` before it adds one, so it never overflows, even for a
range that ends at the largest `int`.

```rust
fn main() {
    let big = 9223372036854775807;
    let mut last = 0;
    for i in big - 2..big {
        last = i;
    }
    print(last - big); // expect: -1
    let r = 3..8;
    print(r.start() + r.end()); // expect: 11
}
```

A range whose start isn't before its end is empty, as in Rust and Python.
That keeps `0..n` total: with `n` from anywhere, it can't fail, and so
neither can a loop over it. `5..4` isn't wrong, only another way to write
the empty range, as `2/4` is another way to write `1/2`. Written with
constants, though, it's most likely a countdown that won't run, so it's a
warning:

```rust
fn main() {
    for i in 5..0 {
        print(i);
    }
    print(1); // expect: 1
}
// warning: this range is empty: `5` is after `0`, so it never yields anything
```

`Range`'s fields are private: only `a..b` and `Range::new` make one. So its
representation is the library's to change, to a start and a length, say,
or to a type that proves `start <= end` once refinement types can say so
(step 8). It's generic, `Range<T>`: a range of anything ordered is a range,
though only `Range<int>` is an iterator for now. It isn't `Copy`: a copy of
an iterator that's advanced is easy to mistake for the iterator itself.

## `where` clauses

A bound can go after a function's signature, and there it can be on `Self`,
or on an associated type, as well as on a type parameter:

```rust
trait Source {
    type Out;

    fn get(&mut self) -> Self::Out;

    // Only for a source that may be thrown away, whose items may be too.
    fn skip(mut self, n: int) -> int where Self: Discard, Self::Out: Discard {
        let mut i = 0;
        while i < n {
            let _x = self.get();
            i = i + 1;
        }
        n
    }
}

#[derive(Discard)]
struct Nat { n: int }

impl Source for Nat {
    type Out = int;

    fn get(&mut self) -> int {
        self.n = self.n + 1;
        self.n
    }
}

fn main() {
    print((Nat { n: 0 }).skip(5)); // expect: 5
}
```

A `where` clause means what the same bound in `<..>` would: inside the
function it's assumed, and at each call it's proved. In the dictionary
reading of traits (docs/06-traits.md), it's evidence the caller passes. So
a provided method like `fold`, which throws the iterator away at the end,
can only be called on an iterator that may be thrown away:

```rust,error
struct Lines { left: int }

impl Iterator for Lines {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.left == 0 {
            return Option::None;
        }
        self.left = self.left - 1;
        Option::Some(self.left)
    }
}

fn main() {
    print((Lines { left: 3 }).fold(0, |a, b| a + b));
}
// error: `fold` needs its `Self` to implement `Discard`, and `Lines` doesn't
```

An impl's version of a method has the trait's `where` clause exactly, at
the impl's type. An impl for a type that doesn't meet it can still define
the method. Its body is then checked assuming something false, which is
harmless, as nothing can call it.

Without them, every bound would have to be on the trait (`trait Iterator:
Discard`), forcing it on every iterator, or a method would need its own
copy of the trait.

## Two gaps that shaped the library

- **`Map` has a parameter no field holds.** `Map<I, F, U>` is what `map`
  gives: `F` of what `I` gives, which are `U`s. Its impl must say what `U`
  is from the type alone (docs/06-traits.md), since a parameter decided only
  by an equality bound, `F: FnMut(I::Item) -> U`, is something resolution
  can't look up yet. The extra parameter is the price.
- **There's no `filter`.** Its impl needs `P: FnMut(&T) -> bool`, a closure
  that takes a reference of any lifetime: a higher-ranked one. A closure's
  signature isn't higher-ranked unless its type is written out, even where
  a bound says it should be (docs/06c-closures.md), so no closure would ever
  meet that bound. `filter` comes when closures can be: step 6e.1,
  docs/06e.1-higher-ranked-closures.md, where it's arrived.

## Running it

The library's files are built into the compiler (`load::CORE_FILES`), read
after a program's own, and shown as `<core>/option.t` and so on. Its
warnings are its own business, so none are shown. What a program doesn't
use of it isn't dead code.

Monomorphisation makes a copy of a generic item only where it's used, and
keeps the library's non-generic ones, like `Ordering`, only if the program
uses them. So `t mono` shows what a program uses of the library and
nothing else. In an editor, hovering over a library item shows its doc
comment. There's no file to go to, so going to its definition goes
nowhere, and it can't be renamed.

## Design decisions worth noticing

- **The library is T.** The checker trusts one intrinsic. Everything else
  in the library is checked like a program, and a bug in it is a type error,
  not a hole.
- **The prelude comes last.** What a module says itself always wins, so the
  prelude can grow.
- **Bounds on methods, not types.** `Option<Fd>` works. Only `unwrap_or`,
  which would drop something, asks for `Discard`.
- **`for` is sugar.** A rewrite into a `loop` needs no rule of its own, and
  can't disagree with the rules it's made of.
- **A total order, and only that.** `Ord` is total, and has no
  `PartialOrd` above it: T has no floats, so there's nothing only partly
  ordered to compare yet. Its laws, like consistency with `==`, are in its
  doc comment. Nothing relies on them to be sound, so a wrong impl gives
  wrong answers, not undefined behaviour. `<` stays on `int`: no operator
  means a trait's method yet.
- **Empty, not invalid.** A reversed range is empty, so `0..n` is total; a
  stricter range is a refinement type, later.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `unwrap`, `expect`, `panic` | panics aren't in function types yet | 7 |
| `filter` | closures can't be higher-ranked when a bound says they should | 6e.1 ✅ |
| `IntoIterator`, `for x in &list` | needs equality bounds with projections in them, and there are no collections yet | with `alloc` |
| `a..=b`, `a..`, `..b`, `.rev()` | not needed yet | — |
| `<` on any `Ord`, `#[derive(Ord)]` | would make `Ord` part of the language | — |
| `Clone`, `Default`, `Show` | `Show` needs strings | — |
| a local type deeper in counting for orphans, `impl Tr for Option<Mine>` | the strictest rule can be relaxed | — |
| a non-zero integer, so `/` can't panic | refinement types | 8 |

## Exercises

1. Write `Option::or`, which gives `self` if it's `Some` and else `other`.
   Which bounds does it need, and why don't they go on `Option` itself?
2. Desugar a `for` loop with a label and a `continue 'outer` by hand, and
   check it does the same.
3. Delete the `start < end` check in `Range::next` and write a loop it
   makes overflow. Why does checking first fix it?
4. Write `swap` with `replace`, or say why you can't.
5. Write an impl that the orphan rule rejects, and a program in another
   package that it could make wrong if it didn't.
6. `Iterator` could say `type Item: Discard;`. Why doesn't it? Write an
   iterator that would rule out, and say where the bound goes instead.

## Reading

- The Rust reference, *Preludes*, *Loop expressions* (the `for`
  desugaring), and *Trait and lifetime bounds* (`where` clauses); Rust RFC
  1023 (orphan rules) and RFC 2451 (re-rebalancing coherence).
- Philip Wadler and Stephen Blott, *How to make ad-hoc polymorphism less ad
  hoc* (1989), on dictionaries: a `where` clause is one more argument.
- Joe Duffy, *The Error Model* (2016), on why `unwrap` without a type that
  says it may fail is a trap.
