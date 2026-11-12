# T language reference

T combines explicit function types, local inference, ownership, and checked
effects. Refinements let the checker prove that arithmetic and indexing are
safe. This reference describes the current language; the
[numbered design notes](../README.md#roadmap) explain how its rules developed.

- [Programs and types](#programs-and-types)
- [Functions and inference](#functions-and-inference)
- [Ownership and borrowing](#ownership-and-borrowing)
- [Closures and generic bounds](#closures-and-generic-bounds)
- [Effects](#effects)
- [Visibility and modules](#visibility-and-modules)
- [Refinements and indexing](#refinements-and-indexing)
- [Errors and libraries](#errors-and-libraries)

## Programs and types

Run a file with `t file.t`, or check it without running it with
`t check file.t`. A program starts at `fn main`. It can take no arguments,
or receive `std::Os`, which supplies its input and output streams, its
arguments (`os.args`) and environment (`os.env`), and the right to start
threads. `t file.t a b` runs it with arguments `a` and `b`.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line("Hello, T!") // expect: Hello, T!
}
```

Blocks are expressions: their last expression supplies their value. A
semicolon discards that expression's value, which requires `Discard`.
An empty block has type `()`. Conditions have type `bool`; `if` branches
and `match` arms must agree on their result type. A `match` must cover
every possible value. `return`, `break`, `continue`, and calls that never
return have type `!`, which can occur wherever a value is expected.

In a pattern, a name that starts with an uppercase letter is always a
path to a variant, never a new variable: `None`, `Some(x)`, `Dot(p)`. One
that names nothing in scope is an error. So no variable's name (in a
`let`, a pattern, a parameter of a function or closure) can start with an
uppercase letter, and every variant's name must. A variant is named with
its enum, `Shape::Dot`, or by itself where a `use` brings it in,
`use Shape::{Dot, Empty};`. The prelude brings in `Some`, `None`, `Ok` and
`Err`. See [variants](45-variants.md).

`if let p = e { .. } else { .. }` runs its block when the pattern matches,
and `while let p = e { .. }` loops while it does. `let p = e else { .. };`
binds the pattern for the rest of the block; its `else` must leave the
block with `return`, `break`, `continue`, or a call that never returns.
Each means the `match` it abbreviates, whose other arm takes apart the
variants the pattern misses, so a value the `else` cannot name must be
`Discard`. See [`if let`](28-if-let.md).

```rust
fn first(s: &str) -> Option<char> {
    let Some(c) = s.chars().next() else { return None };
    if let 'a'..='z' = c { Some(c) } else { None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&first("tea").is_some()) // expect: true
}
```

| Type | Meaning |
|---|---|
| `i8`, `i16`, `i32`, `i64` | Signed integers of the stated width |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers of the stated width |
| `isize`, `usize` | Signed and unsigned integers of the machine's width |
| `bool`, `char`, `()` | Booleans, Unicode scalar values, and the unit value |
| `(A, B)`, `(A, B, C)`, .. | A tuple: two or more values of these types |
| `&T`, `&mut T` | Shared and exclusive references |
| `[T; N]` | An array with a length in its type |
| `&[T]`, `&mut [T]`, `&str` | Borrowed slices and UTF-8 text |
| `fn(T) -> U` | A function pointer, with any effects written after `U` |

Structs have named fields; enums have named variants that may carry
values. A tuple is a struct no one declared, with exported fields `0`,
`1`, ..: `(a, b)` makes one, `t.0` reads a field, and `(x, y)` matches
one, in any pattern and as a closure's parameter, `|(i, x)| ..`. Two
tuple types are equal when their types are, in order. A tuple is
`Copy`, `Discard`, `Eq`, `Ord` (in order, field by field), `Debug`,
`Send` or `Sync` when all its values are. `()` is the tuple with none;
there are no tuples of one, and `(e)` only groups. See
[tuples](58-tuples.md). Declare type parameters with `<T>`, effect parameters with
`<effect e>`, and array lengths with `<const N: usize>`. Slices and `str`
are used behind references. Owned containers such as
`alloc::boxed::Box<T>`, `alloc::list::List<T>`, and
`alloc::string::String` come from libraries and need an import or a full
path.

Arithmetic operands have the same integer type; T does not widen them
implicitly. `+`, `-`, `*`, `/`, `%`, and integer `as` conversions produce
their mathematical result or panic if it is invalid or out of range.
They never silently wrap. Explicit wrapping operations are in `core::num`.
Integer literals support suffixes such as `42i64`, hexadecimal `0xFF`,
and binary `0b1010`. Text uses `"..."` and `'x'`; byte literals use
`b"..."` and `b'x'`. Escapes are `\n`, `\r`, `\t`, `\\`, `\0`, `\'` and
`\"`; `\u{20AC}` in text; and `\xFF` in bytes, or in text up to `\x7F`,
the ASCII characters. A format string, `f"x = {x}, p = {p.y:?}"`, is a
value that's `Display`: its text, with each hole written by `Display`, or
by `Debug` with `:?`. A hole holds an expression, which it borrows, as
`&e`; it ends at the first `}` or `:` outside brackets of its own, and
can't hold a `"` or a comment. `{{` and `}}` are braces (see
[format strings](43.1-formatting.md) and [holes](44-runs-and-holes.md)).

Proofs involving `usize` and `isize` must hold on both 32-bit and 64-bit
machines. A literal of either type must fit on both. See
[integers](11-integers.md), [bits](12-bits.md),
[strings](13-strings.md), and [arrays](22-arrays.md) for the detailed rules.

A `const` item names a value fixed while compiling. Its type is written,
and is plain data: integers, `bool`, `char`, `()`, `&str`, `&[u8]`, and
arrays, tuples, structs and enums of those, with no `Cell` or `Atomic` in them
(they're *frozen*). The initializer may call functions
and loop, but may only panic: no allocation and no `unsafe`. It runs on a
32-bit and a 64-bit machine, which must agree, and a panic there is a
compile error. A const can be an array length, where it is equal only to
itself, and can be named in refinements. See [consts](36-consts.md).

```rust
const SIZE: usize = 4;

fn last(a: &[u8; SIZE]) -> u8 { a[SIZE - 1] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [9u8; SIZE];
    out.line(&last(&a)) // expect: 9
}
```

A `static` item names one place for the whole run, which every function
can reach. Its type is written, and is plain data with atomics in it:
not frozen (else it's a const), `Sync` and `Discard`. Its value is worked
out while compiling, as a const's is. A use of its name is that place,
behind a `&'static`: no `&mut`, no moving out, no assigning; it changes
only through atomics in it. Each use has the effect `global`. See
[statics](38-statics.md).

```rust
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    let _ = COUNT.fetch_add(2);
    out.line(&COUNT.load()) // expect: 2
}
```

## Functions and inference

Named functions declare their parameter types and result type. Omitting
`-> T` means `-> ()`; it does not infer the result from the body. Named
functions may be recursive or nested in blocks. A nested `fn` sees items
and its own parameters, but cannot capture the surrounding function's
local variables.

Local variables, closure parameters, closure results, and generic call
arguments can be inferred from their uses. Annotations constrain that
inference; the compiler reports conflicting or unresolved constraints.

```rust
fn identity<T>(value: T) -> T { value }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let twice = |x| x + x;
    let n = identity(3i64);
    out.line(&twice(n))?; // expect: 6
    Ok(())
}
```

The closure `twice` has one inferred parameter type here, `i64`. A `let`
binding does not make a function generic; declare a generic `fn` to use
one implementation at several types. Explicit call arguments use
`identity::<i64>(3)`, with `_` available for arguments to infer.

Integers have no default type. Comparing two unsuffixed literals or
printing an integer does not choose its width:

```rust,error
fn main() {
    let x = 5;
    let _same = x == 5;
}
// error: cannot infer the type of this integer
```

Use `let x: i64 = 5` or `let x = 5i64` when the surrounding code does not
determine the type. See [inference](03-inference.md) and
[generics](04-generics.md).

## Ownership and borrowing

Using a value by value moves it unless its type implements `Copy`.
After a move, the old place cannot be read until it is initialized again.
`Copy` permits implicit copying and requires `Discard`. Built-in scalar
types and shared references are `Copy`; `&mut T` is not.

`Discard` permits a value to be dropped, overwritten, or ignored.
A value whose type is not `Discard` must be consumed on every path that
continues, including early returns and the early exit of `?`. A leading
underscore and `let _ = value` do not exempt a resource from this rule.
Moving it transfers that obligation; taking it apart by value consumes
it and leaves its fields' obligations to the code that receives them.

```rust
struct Ticket { id: i64 }

fn inspect(ticket: &Ticket) -> i64 { ticket.id }

fn finish(ticket: Ticket) -> i64 {
    let Ticket { id } = ticket;
    id
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let ticket = Ticket { id: 7 };
    let seen = inspect(&ticket);
    let finished = finish(ticket);
    out.line(&seen)?;     // expect: 7
    out.line(&finished)?; // expect: 7
    Ok(())
}
```

The ticket is consumed before either I/O call can return early through
`?`. An error path must not leave it unused.

For ordinary data, `#[derive(Discard)]` allows dropping all fields.
`#[derive(Copy, Discard)]` also permits copying when the fields support it.
`#[derive(Eq)]` compares field by field, `#[derive(Ord)]` orders field by
field (an enum by its variants first) and needs `Eq`, and
`#[derive(Debug)]` writes the value for debugging, `Point { x: 1, y: -2 }`. A bare name in
`#[derive(..)]` is always core's trait.
Derive requirements on generic parameters follow from the fields; exported
types with hidden fields must state the promised bounds explicitly. A
derived `Discard`, `Copy`, `Send` or `Sync` needs only the same trait of
the fields: the type's own bounds, such as `F: Fn() -> i64`, hold
wherever a value of it is made, and aren't asked again.
`Copy` is derived, rather than implemented by hand. A custom
`impl Discard for T` can define `fn discard(self)`; that method must return
`()` and have no effects. Variables drop in reverse declaration order;
fields drop in declaration order, after a custom `discard` has run.

Bindings are immutable unless declared `mut`. A shared reference `&T`
permits reading, of everything reached through it: a `&mut` behind a `&`
is shared too, so `**a` of an `a: &&mut T` can't be written or lent
`&mut`. An exclusive reference `&mut T` permits mutation while
preventing conflicting access. Mutating through a `&mut T` does not
require the binding holding the reference to be `mut`. Borrows last as
long as their uses require; a destructor that reads a borrow counts as
a use too.

References must outlive all their uses. In signatures, write relationships
such as `fn first<'a>(x: &'a i64, y: &i64) -> &'a i64` when they cannot
be expressed by elision. Each omitted input lifetime is distinct; an
omitted output lifetime uses `self`'s reference lifetime in a method, or
the single input lifetime when there is exactly one. Other ambiguous
outputs require a named lifetime. A reference to a local cannot escape
the call that owns it.

The values that change behind a `&` are what's in a
`core::cell::Unsafe<T>`, the one primitive: `as_mut(&self) -> &mut T` and
`as_ref(&self) -> &T` lend what's in it, and are `unsafe` (nothing else may
reach it while the reference lives); `new`, `into_inner` and `get_mut`
(through `&mut`) are safe. `Unsafe` is invariant in `T`, not frozen, `Send`
if `T` is and never `Sync`; a type with one in it gets all of that from its
fields, unless it says `unsafe impl Sync`. The cells of core and alloc are
structs over it: `core::cell::Cell<T>`, `core::sync::Atomic<T>` and
`Mutex<T>`, `core::volatile::Volatile<T>`, `alloc::arena::Arena<T>`.
A cell's:
`get` copies it out (`T: Copy`), `set` replaces it and drops the old one
(`T: Discard`), `replace` swaps it, `into_inner` and `get_mut` work as
for any value. Nothing gives a reference into a cell from a `&Cell<T>`,
and no measure or refinement can read one.
An atomic holds an integer or `bool` (`T: Atom`), changed by `load`,
`store`, `swap`, `compare_exchange` and `fetch_add` and the like, each one
sequentially consistent step.

`Send` (may be handed to another thread) and `Sync` (may be shared by
threads through `&`) are, like `Copy` and `Discard`, derived or built in:
`&T` is `Send` and `Sync` when `T` is `Sync`, a closure when what it
captures is, a `Cell` is `Send` but never `Sync`, an `Atomic` is both.
Or promised, against the fields, by `unsafe impl<T: Send> Sync for
Mutex<T> {}`: only of the two, only in the type's own package, for the
type with any parameters, needing only `Send` or `Sync` of them.

`os.threads.spawn(f)` starts a thread running closure `f` (`Send`, with a
`Send` result), with `f`'s effects and `alloc`, and gives a
`JoinHandle<'a, R>`, which isn't `Discard`: `join` must use it up, and
gives `f`'s result. So `f` may borrow what lives as long as the handle.
`core::sync::Mutex<T>` is `Sync` when `T` is `Send`; `m.lock(|x| ..)`
lends `x: &mut T` to the closure while no other thread has it.

See [ownership](05-ownership.md), [linear values](06a.1-linear-types.md),
[destructors](20-destructors.md), [cells](37-cell.md),
[statics and atomics](38-statics.md), and [threads](41-threads.md).

## Closures and generic bounds

A closure captures the variables it uses. It borrows them by shared
reference, borrows them exclusively when needed, or takes ownership when
its body moves them. `move || ...` captures by value; a `Copy` value is
copied into the closure. Capturing does not require heap allocation.

Each capturing closure has its own type. It implements `Fn` when calls
only need shared access to it, `FnMut` when they need exclusive access,
and `FnOnce` when a call consumes it. `Fn` also supports the `FnMut` and
`FnOnce` interfaces. A closure owning a value that cannot be discarded
is `FnOnce` and must use that value up. A closure without captures can
be used as a function pointer. Return a capturing closure with an opaque
result such as `-> impl Fn() -> i64`.

Generic code can use only the operations its bounds permit. `T: Copy`
permits copying `T`; `T: Discard` permits ignoring it. Associated types
such as `I::Item` come from trait bounds. Closures' effects are inferred
from their bodies and constrained by the bound at a call:

```rust
fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 7;
    out.line(&apply(|| n))?; // expect: 7
    Ok(())
}
```

`Discard` is needed here because calling `f` through `Fn` borrows it,
and `apply` then drops it. `F: Fn() -> i64` without `with e` permits no
effects from the call. A closure parameter written `x: &i64` can accept
a reference of any lifetime; an omitted result lifetime is inferred from
what its body returns, then checked. Returning a reference does not
extend the lifetime of what it borrows.

An impl's bounds on its own type parameters may be of any trait. A trait
impl's bound on anything else, like `where I::Item: Show` or one on a
parameter that another bound decides, may only be of `Discard`, `Copy`, the `Fn` traits,
`Int`, `Signed` or `Unsigned`; make the item type a parameter of the type
for another. A `sealed trait` is implemented only in its own package, so
an impl that needs it doesn't overlap an impl for a type that has none:
`Int` is sealed, so `impl<T: Int> Tr for T` and `impl Tr for bool` can
both exist.

See [traits](06-traits.md), [closures](06c-closures.md),
[opaque results](06c.1-opaque-types.md),
[closure lifetimes](06e.1-higher-ranked-closures.md), and
[closed and sealed traits](29-closed-and-sealed-traits.md).

## Effects

A function's effects say what calling it may do. Write them after the
result type: `with panic`, `with alloc`, or `with {alloc, panic}`.
Effect parameters and associated effects can also occur in these sets.
A bound can say what an associated effect may be at most, `I:
Iterator<Item = u64, e = {}>` or `-> impl Iterator<Item = u64, e = panic>`:
each use must show it, and inside, `I::e` is still only itself, replaced by
its bound only where its effects must be within others. Not in an impl's
bounds.
Every caller must permit the effects of the calls it makes.

| Declaration | Effect rule |
|---|---|
| Non-exported function or inherent method, no `with` | Infer effects from its body |
| Exported function, no `with` | Permit no effects |
| Trait method or trait impl method, no `with` | Permit no effects; impls cannot exceed the trait's contract |
| Any function with `with {}` | Explicitly permit no effects |
| Any function with a written effect set | Check its body against that set |

Inference applies to private and `pub` functions, nested functions, and
`main`. Measures have their own restriction to no effects. Exported
functions and trait interfaces keep an explicit contract: changing a
body cannot silently add an effect to that contract.

```rust
pub fn increment(n: i64) -> i64 { n + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&increment(4))?; // expect: 5
    Ok(())
}
```

Both functions infer `panic` here. Changing `pub` to `export` would
require `with panic` on `increment`. An explicit empty set restricts a
private function too:

```rust,error
fn increment(n: i64) -> i64 with {} { n + 1 }

fn main() {
    let _ = increment(0);
}
// error: `+` may panic, but `increment` doesn't say `with panic`
```

Omitting `with {}` on this private function would infer `panic`.
Keeping it asks the checker to prove the addition safe for every `i64`,
which it cannot do. A refinement or a guard can supply the missing bound.

`panic` covers invalid arithmetic, explicit panics, and operations such as
`unwrap` that may panic. `alloc` covers allocation and aborting on out of
memory; it does not require `panic` as well. `with {}` also excludes
allocation, but does not promise termination.

`global` is using a `static`: state no argument reaches. A function
without it (and without `unsafe`) touches only what its arguments reach.

`unsafe` marks operations whose callers must uphold obligations the
checker cannot establish. It is never inferred: declare `with unsafe`
to pass the obligation to a caller, or use an `unsafe { ... }` block
where the obligation is upheld. Such a block discharges only `unsafe`;
any `panic` or `alloc` inside still contributes to the surrounding
function's effects.

See [effects](07-effects.md), [effect inference](24-visibility.md#effects-of-what-isnt-exported),
[allocation failure](16-out-of-memory.md), and [unsafe code](09-unsafe.md).

An `extern fn` has C's calling convention, under its own name. With no
body it's an import, a C function: a call has the effects it declares
and `unsafe`, and its result can't be a refinement. With a body it's an
export, a T function C can call too: it can't say `with unsafe`, its
parameters can't be refinements, and its effects are at most `global` and
`panic`, as C may call it from an interrupt in the middle of an
allocation. Only integers, `bool`, and as a result `()` and `!`, cross; an
`extern fn` has no generic parameters, and only a program, not a library,
can have one. Its C name is its own name, wherever it's declared. The
interpreter (`--interp`) stops at an import's call; compiled, it's
called. A program that writes the platform's `t_platform_write` or
`t_platform_halt` is checked as any other, but runs, and builds, only for
the freestanding target. `core::volatile::Volatile<T>` is a
`T: Atom` at a fixed address: `Volatile::at(address)` is `unsafe`, and
`read` and `write` on what it gives are safe volatile accesses. See
[`extern fn`](40-extern.md).

```rust,error
extern fn abs(x: i32) -> i32;

fn main() {
    let _ = abs(-3);
}
// error: `abs` is unsafe, but it isn't in an `unsafe` block
```

## Visibility and modules

Visibility and effects are separate choices. `pub` exposes an item within
its package and still permits effect inference. `export` promises an
interface to other packages and requires declared effects.

| Visibility | Who can use the item |
|---|---|
| No keyword | Its defining module and descendants |
| `pub` | Any module in its package |
| `export` | Other packages too |

Every segment of a path must be visible. Exported items must also be
reachable from the package root through exported modules or `export use`
names. A re-export cannot widen the visibility of its target. Types and
traits named in an item's signature must be visible at least as widely
as the item.

Struct fields and inherent methods have their own visibility, bounded
by their type's visibility. A struct with hidden fields cannot be
constructed or taken apart outside the boundary that hides them.
An enum's variants share its visibility, including when a `use` brings
them in by themselves; trait methods share their trait's interface.

Modules do not implicitly import the surrounding module's names. Use
`self::`, `super::`, or `crate::` paths, or a `use` declaration; the core
prelude is available in every module after local declarations and imports.
For example:

```rust
mod numbers {
    pub fn answer() -> i64 { 42 }
}

use crate::numbers::answer;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&answer())?; // expect: 42
    Ok(())
}
```

For several source files, the root must be named `main.t`. A declaration
`mod a;` loads `a.t`; module `a::b` lives in `a/b.t` relative to the
root's directory, including when `a` was declared inline. There is no
`mod.t` alternative. These files mean the same as inline module bodies.

See [visibility](24-visibility.md), [module names](06d-modules.md), and
[source files](06d.1-files.md).

## Refinements and indexing

`{ v: T | condition }` restricts a value of type `T`. Refinements can be
the whole type of a function parameter, result, single-variable `let`,
or struct or variant field, or a type argument there of any type, at any depth, such
as `Option<{ v: i64 | v > 0 }>` or `List<{ s: u8 | s < 64 }>`: each value
at that position satisfies it (each field of a variant whose type is that
parameter, when the value is that variant). See [refined type
arguments](60-refined-type-arguments.md). They can refine any type, using
integers, booleans, and measures in their conditions. They cannot appear
on closure parameters or trait methods. An impl's method of a trait can
refine its result and add `ensures`, which calls the checker knows are of
that impl (their `Self` type has no type parameters) can rely on; its
parameters cannot be refined.

Predicates support linear arithmetic, comparisons, and boolean
combinations. Multiplication of at most two unknown values is supported;
`/` and `%` only by a constant other than 0, of literals, rounding
towards zero as in code ([division](72-division.md)). Predicates
reason about mathematical integers, without overflow. Actual program
arithmetic still has to fit its integer type.

Parameter predicates may mention earlier parameters; result predicates
may mention parameters. A field predicate may mention earlier fields
with exactly the same visibility. A variant's field predicate may
mention the fields before it by their binders, `Range({ lo: u8 | lo <
100 }, { hi: u8 | hi >= lo })`: making the variant proves them, and a
`match` knows them. Construction and assignment must
preserve every affected field refinement. A refined variable, or a field
that a field refinement constrains, cannot be borrowed mutably, except
straight into a call that can't keep the reference: then the refinement
is proved again after the call. A field whose refinement is about it
alone (it names no other field, and no other field's names it), a
struct's or a variant's, can be: the reference says the refinement, so
what's written through it must have it.

A type alias can stand for a refined type, or have one in it, `type
Square = { s: u8 | s < 64 };`, `type Moves = List<Square>;`, and its
predicates can name its const parameters, `type Below<const N: usize> =
{ i: usize | i < N };`. A use means the refinement written out there,
so it can be only where a refinement can be; an alias's argument's
refinement goes where its parameter is. Its predicates name only their
binders, its const parameters and consts, as they are where it's
declared, and are checked there, for any arguments. See [refinement
aliases](67-refinement-aliases.md). A field of a value whose refinement is
about the whole of it (a refined variable, or what's declared of a whole
tuple element, list or array element, box's value or `&mut`'s pointee) can
be written only if the whole keeps its refinement, and cannot be borrowed
mutably. A pattern's binding by `&mut` (match ergonomics, `match o {
Some(x) => .. }` for an `o: &mut Option<..>`) is a `&mut` of the place it
binds, a variant's field or an array's element too: what's written
through it must have what's declared there, and it's refused where a
`&mut` of that place would be, or where a struct field's own refinement
says something of the variant's field it's in. See [one
walk](66-one-walk.md).

Every reference says what's declared of what it points to, whatever it's
made from (an element, `&mut l[0]`; a field of one; what a call's
reference points to) and however it's kept or passed on: what's written
through it must have that, and where it's given to a function, the
function's type must say exactly that behind it (`&mut T`, `IterMut<'_,
T>` and `Cell<T>` are invariant in `T`). A refined field of a temporary
can't be lent `&mut`, even to a call. What an iterator adapter gives
(`skip`, `rev`, `enumerate`, `zip`, `copied`) has what its iterator's
type says of its values. A closure or function given where the callee
will pass it a `&mut` to something refined must be a closure written in
the call, which the callee doesn't keep (not `map`'s). See [what's
declared](63-what-is-declared.md).

A predicate can read a struct's field from anything it names, at any
depth and through references and boxes: `v == p.x`, `s.0.len() ==
mid`. The field must be visible there, and one named in a signature's
refinement must be visible at least as far as the function (exported for
an impl's method of a trait); one behind a `&mut` follows the same rules
as a measure of what's behind it. `s.slice(a..b)` and `slice_mut` give a
sub-slice of a slice, an array or a list for a range proved in it, and
`split_at` and `str::get` say how long what they give is. See
[field paths and sub-slices](56-field-paths-and-slices.md).

A measure is a method marked `#[measure]` that takes only `&self`, returns
an integer (or a type parameter `T: Int`) or boolean, has no effects, and
has a body accepted by the refinement logic. Measures such as a slice's `len()` connect values to
proofs. Indexing calls `index` or `index_mut`; the standard containers
require the checker to prove that the `usize` index is in range.

```rust
fn first(s: &[i64]) -> Option<i64> with {} {
    if s.len() > 0 { Some(s[0]) } else { None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let values: [i64; 2] = [5, 9];
    out.line(&first(values.as_slice()).unwrap_or(0))?; // expect: 5
    Ok(())
}
```

Guards, refined signatures, assignments, patterns, and type ranges supply
facts. In `for x in a..b`, `x < b` is known. A call of a generic function
keeps what's known of its arguments' values of a *free* type parameter,
one only marker traits (`Copy`, `Discard`, `Send`, `Sync`) bound, in no
other bound, and only whole or in enums' fields in the signature: each in
the result is one of those (`o.unwrap()` is what's in `o`, and `?` gives
what's in the `Some` or `Ok`, so a function with a refined result can use
it). See [free theorems](55-free-theorems.md).
Mutation, mutable lending, and loop iterations can invalidate facts
about a previous value. Capture a current value and check it again when
needed. At a loop's head, what the loop changes is forgotten except what
its calls ensure stays the same and what its `invariant p` says (`while
lo < hi invariant hi <= a.len() { .. }`, also on `loop`, `for` and `while
let`): `p` must hold as the loop starts, at the end of its body and at
each `continue`. After a loop, what held at one of its exits (its
condition false, or a `break`) holds. See [loop frames](52-loop-frames.md)
and [loop invariants](54-loop-invariants.md). Diagnostics explain the required condition, relevant known
facts, and mutations that caused facts to be forgotten.

`<` of two `char`s compares their numbers, so a guard such as `c < 'a'`
is a fact about `c as u32`. A tuple or struct pattern says which variants
its parts are, so `(Some(a), Some(b))` knows what the options' types say
of `a` and `b`.

Proof search is bounded and incomplete. A failed proof may mean a missing
fact, unsupported reasoning, or an arithmetic or complexity limit; it
does not establish that the condition is false. Every proof that fails is
reported, not only the first: after reporting one, checking assumes it
held. Any other error stops checking. `with panic` permits an
unproved arithmetic operation to panic, but does not bypass a refinement
on an argument or the proof required by indexing. The lint
`implicit_panics`, off unless `#[warn(implicit_panics)]` turns it on,
warns of each operation whose check wasn't proved. Use `get`, which returns
an `Option`, when an index cannot be proved valid.

See [refinements](08-refinement-types.md),
[field invariants](08.1-field-refinements.md), and [measures](21-measures.md).

## Errors and libraries

Recoverable errors are values. `Result<T, E>` carries a success or error;
`Option<T>` carries a value or its absence. `e?` continues with the
success value or returns early from the current function or closure.
The enclosing result must implement `Try` with the same residual type;
there is no automatic conversion between different error types.
Every early exit must also satisfy ownership rules. A panic aborts
without unwinding. `assert(ok)`, `assert_eq(&a, &b)` and `assert_ne(&a,
&b)` panic unless what they check holds; the last two say both values by
their `Debug`. An abort (a panic, or running out of memory) directly in
the body of a function marked `#[blame_caller]` is reported where that
function was called, and further out while the caller is marked too; one
used as a value is blamed where it became a value. core, alloc and std mark
their functions that can abort.

A `#[test]` function takes nothing and gives back `()` or `Result<(), E>`,
as `main` may; `t test` runs each as a program of its own, which passes
unless it panics, aborts or gives back `Err`. `#[test(panics)]` passes only
if it panics. Tests are checked by every command, and are never run by
`t` (see [31-tests.md](31-tests.md)).

`t main.t` and `t test` compile the program through C and run it;
`--interp` runs it in the interpreter, which checks as it goes what the
checker proved. A panic ends a program with status 101; anything else
that fails, with 1 (see [44-runs-and-holes.md](44-runs-and-holes.md)).

Lints are warnings, which `#[allow(..)]` turns off and `#[warn(..)]` on,
on an item, field, variant or `mod`, or, as `#![allow(..)]` and
`#![warn(..)]` at the top of a module, in all of it; the innermost
decides. `format_hole_expressions`, `format_hole_effects` and
`implicit_panics` are off unless turned on. `--deny-warnings` makes `t check` and `t test` fail on
warnings.

I/O requires a stream passed as a value, usually from `main`'s `std::Os`.
It has no separate effect: stream methods report failures as `Result`s,
with the one `core::io::Error`. Code generic over where bytes come from
and go uses `core::io::{Read, Write}`; `core::io::Bytes` and
`List<u8>` are those in memory, and `alloc::io::read_line` reads a line
from any `Read` (see [I/O](47-io.md)). Files are reached through
`os.fs`: whole with `read`, `read_to_string`, `write` and `append`, or
read bit by bit through the `ReadFile` that `open` gives (see
[files](48-files.md)). The time is `os.clock`, unguessable bytes are
`os.entropy`, and `os.stdin.raw()` puts the terminal in raw mode for as
long as the `Raw` it gives lives (see
[time, entropy and the terminal](49-time-entropy-terminal.md)).
A `main` returning `Result<(), E>` reports an error to standard error
and exits unsuccessfully when it returns `Err`; `E` must implement
`core::fmt::Display` and `Discard`.

| Library | Common items |
|---|---|
| [`core`](../core/lib.t) | `Option`, `Result`, `Iterator`, `cell::Cell`, `sync::Atomic`, `volatile::Volatile`, `time::Duration`, `io::{Read, Write, Error}`, numeric operations, traits, and formatting |
| [`alloc`](../alloc/lib.t) | `boxed::Box`, `list::List`, `string::String`, `arena::Arena`, and `io::read_line` |
| [`std`](../std/lib.t) | `Os`, `io` streams and `io::Raw`, `env::{Args, Env}`, `fs::{Fs, ReadFile}`, `time::{Clock, Instant}`, and `entropy::Entropy` |

The compiler embeds these libraries. Every package has `core`. A
package with a `t.toml` beside its root has only the other libraries
that file lists, as `libraries = ["alloc", "std"]`, and a path starting
with one it doesn't list is an error; a program with no `t.toml` has all
three (see [libraries](35-libraries.md)). The
[`core` prelude](../core/prelude.t) supplies names including `Option`,
`Result`, `Copy`, `Discard`, `Iterator`, and the `Fn` traits. Import
allocating containers explicitly, for example `use alloc::list::List;`.
See [error propagation](15-try.md), [formatting](18-fmt.md), and
[I/O](19-std-io.md) for more examples.
