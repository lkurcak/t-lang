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
or receive `std::Os`, which supplies its input and output streams.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"Hello, T!") // expect: Hello, T!
}
```

Blocks are expressions: their last expression supplies their value. A
semicolon discards that expression's value, which requires `Discard`.
An empty block has type `()`. Conditions have type `bool`; `if` branches
and `match` arms must agree on their result type. A `match` must cover
every possible value. `return`, `break`, `continue`, and calls that never
return have type `!`, which can occur wherever a value is expected.

`if let p = e { .. } else { .. }` runs its block when the pattern matches,
and `while let p = e { .. }` loops while it does. `let p = e else { .. };`
binds the pattern for the rest of the block; its `else` must leave the
block with `return`, `break`, `continue`, or a call that never returns.
Each means the `match` it abbreviates, whose other arm takes apart the
variants the pattern misses, so a value the `else` cannot name must be
`Discard`. See [`if let`](28-if-let.md).

```rust
fn first(s: &str) -> Option<char> {
    let Option::Some(c) = s.chars().next() else { return Option::None };
    if let 'a'..='z' = c { Option::Some(c) } else { Option::None }
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
| `&T`, `&mut T` | Shared and exclusive references |
| `[T; N]` | An array with a length in its type |
| `&[T]`, `&mut [T]`, `&str` | Borrowed slices and UTF-8 text |
| `fn(T) -> U` | A function pointer, with any effects written after `U` |

Structs have named fields; enums have named variants that may carry
values. Declare type parameters with `<T>`, effect parameters with
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
`b"..."` and `b'x'`.

Proofs involving `usize` and `isize` must hold on both 32-bit and 64-bit
machines. A literal of either type must fit on both. See
[integers](11-integers.md), [bits](12-bits.md),
[strings](13-strings.md), and [arrays](22-arrays.md) for the detailed rules.

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
    Result::Ok(())
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
    Result::Ok(())
}
```

The ticket is consumed before either I/O call can return early through
`?`. An error path must not leave it unused.

For ordinary data, `#[derive(Discard)]` allows dropping all fields.
`#[derive(Copy, Discard)]` also permits copying when the fields support it.
Derive requirements on generic parameters follow from the fields; exported
types with hidden fields must state the promised bounds explicitly.
`Copy` is derived, rather than implemented by hand. A custom
`impl Discard for T` can define `fn discard(self)`; that method must return
`()` and have no effects. Variables drop in reverse declaration order;
fields drop in declaration order, after a custom `discard` has run.

Bindings are immutable unless declared `mut`. A shared reference `&T`
permits reading. An exclusive reference `&mut T` permits mutation while
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

See [ownership](05-ownership.md), [linear values](06a.1-linear-types.md),
and [destructors](20-destructors.md).

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
    Result::Ok(())
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
    Result::Ok(())
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

`unsafe` marks operations whose callers must uphold obligations the
checker cannot establish. It is never inferred: declare `with unsafe`
to pass the obligation to a caller, or use an `unsafe { ... }` block
where the obligation is upheld. Such a block discharges only `unsafe`;
any `panic` or `alloc` inside still contributes to the surrounding
function's effects.

See [effects](07-effects.md), [effect inference](24-visibility.md#effects-of-what-isnt-exported),
[allocation failure](16-out-of-memory.md), and [unsafe code](09-unsafe.md).

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
An enum's variants share its visibility; trait methods share their
trait's interface.

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
    Result::Ok(())
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
or struct field. They can refine any type, using integers, booleans,
and measures in their conditions. They cannot be nested inside another
type such as `Option<{ v: i64 | v > 0 }>`, or appear on closure parameters
or trait methods.

Predicates support linear arithmetic, comparisons, and boolean
combinations. Multiplication by a constant is supported; multiplication
of two unknown values and division in a predicate are not. Predicates
reason about mathematical integers, without overflow. Actual program
arithmetic still has to fit its integer type.

Parameter predicates may mention earlier parameters; result predicates
may mention parameters. A field predicate may mention earlier fields
with exactly the same visibility. Construction and assignment must
preserve every affected field refinement. A refined variable, or a field
that a field refinement constrains, cannot be borrowed mutably.

A measure is a method marked `#[measure]` that takes only `&self`, returns
an integer or boolean, has no effects, and has a body accepted by the
refinement logic. Measures such as a slice's `len()` connect values to
proofs. Indexing calls `index` or `index_mut`; the standard containers
require the checker to prove that the `usize` index is in range.

```rust
fn first(s: &[i64]) -> Option<i64> with {} {
    if s.len() > 0 { Option::Some(s[0]) } else { Option::None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let values: [i64; 2] = [5, 9];
    out.line(&first(values.as_slice()).unwrap_or(0))?; // expect: 5
    Result::Ok(())
}
```

Guards, refined signatures, assignments, and type ranges supply facts.
Mutation, mutable lending, and loop iterations can invalidate facts
about a previous value. Capture a current value and check it again when
needed. Diagnostics explain the required condition, relevant known
facts, and mutations that caused facts to be forgotten.

Proof search is bounded and incomplete. A failed proof may mean a missing
fact, unsupported reasoning, or an arithmetic or complexity limit; it
does not establish that the condition is false. `with panic` permits an
unproved arithmetic operation to panic, but does not bypass a refinement
on an argument or the proof required by indexing. Use `get`, which returns
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
without unwinding.

I/O requires a stream passed as a value, usually from `main`'s `std::Os`.
It has no separate effect: stream methods report failures as `Result`s.
A `main` returning `Result<(), E>` reports an error to standard error
and exits unsuccessfully when it returns `Err`; `E` must implement
`core::fmt::Display` and `Discard`.

| Library | Common items |
|---|---|
| [`core`](../core/lib.t) | `Option`, `Result`, `Iterator`, numeric operations, traits, and formatting |
| [`alloc`](../alloc/lib.t) | `boxed::Box`, `list::List`, `string::String`, and `arena::Arena` |
| [`std`](../std/lib.t) | `Os` and `io` streams |

The compiler embeds these libraries. The
[`core` prelude](../core/prelude.t) supplies names including `Option`,
`Result`, `Copy`, `Discard`, `Iterator`, and the `Fn` traits. Import
allocating containers explicitly, for example `use alloc::list::List;`.
See [error propagation](15-try.md), [formatting](18-fmt.md), and
[I/O](19-std-io.md) for more examples.
