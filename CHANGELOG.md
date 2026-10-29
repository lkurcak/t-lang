# Changelog

## 0.37.0 (2026-10-29)

Step 24: visibility and effect inference. See
[docs/24-visibility.md](docs/24-visibility.md).

### Added

- `export`: what other packages may use, the package's interface. Every
  exported item must be reachable from the package's root through
  exported modules and `export use`s; what an exported item's signature
  names must be exported too.
- Effect inference: a function, inherent method or nested function that
  isn't exported and says no `with` has the effects its body has, `main`
  too. A recursive group gets the least effects that hold for all of it.
  An error at an exported function says, in notes, why what it calls has
  the effect. `unsafe` is never inferred.
- `with {}` says a function has no effects, where they'd be inferred.

### Changed

- `pub` is seen in its package only. The libraries `export` what they
  had `pub`, but for `std::rt`, which only the runtime uses.
- Written derive bounds are needed only for an exported struct with
  fields that aren't, rather than for a `pub` one with private fields.
- `unused_effects` warns of a `pub` function too: only an exported one
  may say an effect it doesn't have, to keep the right to have it later.
- A field's refinement can name only fields exactly as visible as it,
  of the three levels.
- Exported items are never dead code.
- Bodies are checked callees first, so the first error reported may be
  in a function after the one it used to be in.

### Fixed

- `t fmt` breaks a function's parameters, one to a line, before its return
  type: no more `-> Result<` with the type's arguments on lines of their
  own.
- `std/` is checked to be formatted, as `core/` and `alloc/` are.

## 0.36.0 (2026-10-29)

Step 23: arenas. See [docs/23-arenas.md](docs/23-arenas.md).

### Added

- `alloc::arena::Arena<T>`: values on the heap that live as long as the
  arena and are dropped with it, first put in first. `a.alloc(v)` moves
  `v` in and gives a `&mut` to it, from a `&a`, so many at once, and
  values in an arena can point to each other. It's `Discard` if `T` is,
  and invariant in `T`.

### Fixed

- The drop check now looks into an intrinsic type's arguments: dropping
  one reaches a `discard` of what it holds.

## 0.35.0 (2026-10-25)

Step 22: arrays. See [docs/22-arrays.md](docs/22-arrays.md).

### Added

- Arrays, `[T; N]`: exactly `N` values of type `T`, inline, a value of
  their own. `[a, b, c]` lists them; `[v; n]` is `n` copies of a `Copy`
  `v`; `let [a, b] = arr;` and `match` take one apart, by an array
  pattern of exactly its length, moving its values out.
- Const parameters, `const N: usize`, on functions, structs, enums and
  impls: a length the caller picks, worked out from the arguments or
  written as a literal, `Ring<i64, 8>`, `zeros::<3>()`. In the body and in
  refinements it's a `usize`.
- `core::array`: `len` (a measure, `N`), `is_empty`, `index` and
  `index_mut` (so `a[i]` is proved in range), `get`, `get_mut`,
  `as_slice`, `as_mut_slice`, `iter`, `iter_mut`, and `from_fn`, which
  makes each value from its index.
- An array is `Discard`, `Copy` or `Eq` exactly when its values are, and
  drops them front to back.
- The `non_upper_case_globals` lint, for const parameters: `N`, `CAP`.

### Changed

- Moving out of what a call's reference points to, as `s[i]` of a value
  that isn't `Copy`, says "this value" rather than "`*a temporary`".

## 0.34.0 (2026-10-25)

Step 21: measures. See [docs/21-measures.md](docs/21-measures.md).

### Added

- Measures: a method marked `#[measure]`, of `&self` only, giving an
  integer or a `bool`, with no effects, whose body is a term of the logic
  (`self`'s fields, other measures, literals and arithmetic) or which is
  an intrinsic. A refinement can call one: `i: { i: usize | i < s.len() }`.
  In its module it's its body; elsewhere, unless its body reads no
  fields, it's only what's been learned of it.
- Refinements of any type, which say something of it through its
  measures: `{ s: &[T] | s.len() > 0 }`.
- `x[i]` is `*x.index(i)`, and `*x.index_mut(i)` where it's assigned or
  borrowed `&mut`. `[T]::index` and `index_mut` take `i < self.len()`, so
  they can't panic; `List` has them too.
- Measures in the libraries: `len` and `is_empty` of slices, strings,
  `List` and `String`, `List::capacity`, `RawBuf::cap`. `as_slice`,
  `as_mut_slice`, `as_str`, `as_bytes`, `RawBuf::slice` and
  `from_utf8_unchecked` say how long what they give is.
- The interpreter checks the refinements of intrinsics, and measures in
  any refinement.

### Changed

- A place lent `&mut` is forgotten where it's lent, not treated as
  changing anywhere in the body when the borrow could outlive a call.
- A result's refinement can name what's behind a `&mut` parameter, and
  means it as it was at the call.
- `RawBuf::slice` and `slice_mut` take `hi >= lo`.
- What's assigned is read before a call in the place assigned to, as
  `index_mut` in `s[0] = s[1]`, borrows anything.
- A parameter a refinement names isn't an unused variable.
- `t fmt` keeps a refined result on one line, breaking the parameters
  first, and keeps the blank line before a method with an attribute.

## 0.33.0 (2026-10-24)

Step 20: destructors. See [docs/20-destructors.md](docs/20-destructors.md).

### Added

- `Discard` has a method, `discard(self)`, called wherever a value is
  dropped. A struct or enum may write its own, in an impl in its module,
  needing only `Discard` of its parameters; not for a `Copy` type. It has
  no effects, so it's called on every path, the early ones of `?` too.
- `x.discard()` drops `x` now, for any `Discard` type.
- `List<T>` is `Discard` when `T` is: dropping it drops its values and
  frees its buffer. `String` is `Discard`. So a function holding one can
  use `?`.
- The drop check: dropping a value whose type may reach a `discard` of its
  own uses it, so what it borrows must still be there.

### Changed

- Outside its module, a value whose type has a `discard` of its own can't
  be taken apart, by a pattern or by moving a field out; in its module, a
  by-value pattern uses it up, without calling `discard`.
- tree-sitter 0.27 (was 0.26), for the parser `t tree-sitter` builds.

### Removed

- `List::free` and `String::free`: a list or string is freed when it's
  dropped, or by `x.discard()`. `List::free_with` stays, for values that
  aren't `Discard`.

## 0.32.0 (2026-10-24)

Step 19: std I/O. See [docs/19-std-io.md](docs/19-std-io.md).

### Added

- The `std` library, beside `core` and `alloc`, loaded for every program
  and not in the prelude.
- `fn main(os: std::Os)`: `Os` holds the program's streams, `stdin`,
  `stdout` and `stderr`, intrinsic values no code can make or copy, so a
  function does I/O only through a stream it's handed.
- `Stdout` and `Stderr` are `core::fmt::Write` sinks whose `Error` is
  `std::io::Error`, with `put` and `line` of their own;
  `Stdin::read_line(&mut line)`, `Ok(false)` at the end of input.
- `Write::line(&x)`: `x`, then a line break, for every sink.
- `main` can give back `Result<(), E>`, with `E: Display + Discard`: an
  `Err` is written to stderr, `error: ...`, and the program fails.
- Warning `unused_results`: a statement that throws away a `Result`
  whose error type isn't `!`. The fix is `let _ =`.
- Tests: `// stdin:` lines give a program input, and `// stderr:` lines
  say what it writes where errors go.

### Changed

- `t run` writes a program's output as it runs, not after, and before
  each read of input, so a prompt is seen. Warnings come first.
- The import fix for an unknown type also finds a library's root items:
  `use std::Os;`.

### Removed

- The `print`, `print_bool` and `print_str` built-ins, and printing what
  `main` gives back: a `main` gives back `()` or a `Result<(), E>`.

## 0.31.0 (2026-10-24)

Step 18: `core::fmt`. See [docs/18-fmt.md](docs/18-fmt.md).

### Added

- `core::fmt::Write`, a sink of text: `write_str`, `write_char`, and
  `put(&x)`, which writes anything `Display`. Each sink says how writing
  to it fails, `type Error`, and what it does, `effect e`.
- `core::fmt::Display`, whose `fmt` does only what the sink does: for
  the integer types, in decimal, `bool`, `char`, `str`, `&str`, and
  references to what has it; in alloc, `String` and `Box`.
- `String` is a sink (`type Error = !`, `effect e = alloc`), and
  `String::of(&x)` makes a string of anything `Display`.
- `Result<T, !>::into_ok`: what's in a result that can't be an `Err`,
  with no `panic`.
- Warning `unwrap_infallible`: `unwrap`, `expect`, `unwrap_or` or
  `unwrap_or_else` of a `Result` whose error type is `!`. The fix is
  `into_ok()`.

### Fixed

- A box is alloc's, for the orphan rule: alloc can implement core's
  traits for `Box`, and a program can't, as for any other of alloc's
  types. Before, a box counted as core's.
- An effect a trait impl's method can't have no longer suggests saying
  it, which its trait wouldn't allow; a note says why instead.

### Changed

- Closures' symbols, as `t mono` shows them, have other numbers.

## 0.30.0 (2026-10-24)

Step 17.1: built-ins are library items. See
[docs/17.1-library-items.md](docs/17.1-library-items.md).

### Changed

- `Discard`, `Copy` (`core::mem`), `Eq` (`core::cmp`), `Int`, `Signed`,
  `Unsigned` (`core::num`), `Fn`, `FnMut` and `FnOnce` (`core::ops`) are
  core's traits, declared `#[intrinsic]`, and in scope through the
  prelude. A program may declare a trait by any of their names: it's
  another trait, with none of what core's has.
- `Box` is `alloc::boxed::Box`, and a program that uses it says `use
  alloc::boxed::Box;`. A program may declare its own `Box`.
- `<`, `<=`, `>` and `>=` are `Ord`'s `lt`, `le`, `gt` and `ge`, as `==`
  is `Eq`'s: they compare anything `Ord`, looking at their operands
  without moving them. Of an integer type they're the machine's, as
  before. `Int` needs `Ord`.
- `#[derive(..)]` names its traits as paths do, in scope or by a path,
  and only core's `Discard`, `Copy` and `Eq`. `#[derive(Eq)]` where a
  program's own `Eq` is in scope is an error.
- Messages name the prelude's traits by their names (`Iterator`, not
  `core::iter::Iterator`), unless a program has another trait of that
  name.
- Closures' symbols, as `t mono` shows them, have other numbers, and `t
  mono` writes a box's type and `Box::new` by their paths.

### Added

- `impl Ord for char`, by code point; `Ord` for `&T` when `T: Ord`, and
  for `&str`.
- An unknown type that exactly one module of core or alloc has, like
  `Box`, `List` or `String`, gets a fix that imports it.
- `#[intrinsic]` on a trait, in core and alloc only. When core is
  checked, each built-in impl of an intrinsic trait must have that
  trait's supertraits.
- Hovering a trait shows its doc comment, and hovering `Box::new`'s
  `Box` shows `Box`'s.
- tree-sitter parses paths in `#[derive(..)]`.

## 0.29.0 (2026-10-24)

Step 17: what the first retrospective changed. See
[docs/17-retrospective-1.md](docs/17-retrospective-1.md).

### Changed

- An effect parameter is declared with `effect`: `fn apply<effect e, F:
  Fn() with e>(f: F) with e`. Case no longer decides it, so `fn id<t>(x:
  t) -> t` is a type parameter, with a naming warning, and a `with e`
  naming a type parameter is an error whose message says to declare it
  `effect e`. `panic` and `alloc` can't be effect parameters' names.
- An associated effect's name needn't be lowercase either.
- `Discard` and `Copy` can only be derived: `impl Discard for P {}` and
  `impl Copy for P {}` are errors, whose fix derives them instead. A
  derived one's `where` can only need the same trait.
- A `///` doc comment before `mod` is an error: a module's doc comment
  goes inside it, as `//!`. For an inline module, the fix moves it there.
- Core's `Iterator::map`, `filter` and `fold` call their effect
  parameters `fx` and `px`, not `f` and `p`, the names of their values.
- Closures' symbols, as `t mono` shows them, have other numbers.

### Added

- `//!` doc comments, at the start of a file or of `mod m {`, document
  the module. Hovering a module's name shows them, where it's declared,
  in a `use` and at the start of a path; core's and alloc's files have
  them. Anywhere else, `//!` gets the `unused_doc_comments` warning. `t
  fmt` prints them first in the module, then a blank line; tree-sitter
  highlights them.
- Hovering an effect parameter says it's one; renaming one to `panic` or
  `alloc` is refused.

### Fixed

- The kinds of associated items are checked: an associated type can't
  be used as effects (`with Self::Item`), an associated effect can't be
  a type, and an impl can't give an associated effect with `type e =
  ..;`. Before, these were accepted, with messages like "may have the
  effects `i64`".
- An impl's method must have type and effect parameters of the kinds the
  trait's method has.

## 0.28.0 (2026-10-24)

Step 16: running out of memory is `alloc`. See
[docs/16-out-of-memory.md](docs/16-out-of-memory.md).

### Changed

- Running out of memory is part of the `alloc` effect, not `panic`, which
  now means a bug. `Box::new`, `RawBuf::new`, `RawBuf::grow`,
  `List::with_capacity`, `List::push`, `String::with_capacity`,
  `String::from`, `String::push` and `String::push_str` say `with alloc`,
  not `with {alloc, panic}`. A program that said `panic` only for them
  gets an `unused_effects` warning, whose fix takes it out.
- `RawBuf::grow` takes how many slots to add, not the new size, so it
  can't shrink the buffer and doesn't panic. More slots than a `usize`
  can count is running out of memory.
- Closures' symbols, as `t mono` shows them, have other numbers.

### Added

- `alloc::raw::out_of_memory()`, which aborts as running out of memory
  does, `with alloc`: for a collection that can't count one more value.
## 0.27.0 (2026-10-24)

Step 15: `?`. See [docs/15-try.md](docs/15-try.md).

### Added

- `e?`: the value in an `Ok` or a `Some`, or else return the `Err` or the
  `None` from the function (or closure) it's in. It's rewritten into a
  `match` on `core::ops::Try::branch(e)`, so it has no rules of its own.
- `core::ops::Try`, with `Output`, `Residual`, `branch` and
  `from_residual`, for `Option` and `Result`, and which a program can
  implement for its own types; and `core::ops::ControlFlow<B, C>`.
- A `?` returns early only into a type with the same residual: an
  `Option`'s into an `Option`, a `Result`'s into a `Result` with the same
  error type. Errors aren't converted.
- Errors about a `?` say so: what it's applied to, or what the function
  returns, isn't `Try`; their residuals differ; a value that must be used
  up goes out of scope where the `?` may return. Tree-sitter, highlighting
  and `t fmt` know `?`.

### Fixed

- An impl's method with more effects than its trait's no longer offers a
  fix that edits the trait, when the trait is a library's.

### Changed

- Closures' symbols, as `t mono` shows them, have other numbers, as the
  core library has more in it.

## 0.26.0 (2026-10-24)

Step 14: checking packages separately. See
[docs/14-packages.md](docs/14-packages.md).

### Changed

- The core and alloc libraries are checked once, each against those
  before it (core alone, then alloc), and every program's checking starts
  from what that left: only the program's own code is checked for it.
  After the first program in a process, checking `fn main() {}` takes
  about 4 ms rather than 29 ms; the test suite takes half as long.
- An error that only a program's code together with a library's makes
  is reported in the program: an impl whose bounds let a library impl's
  loop (as `impl<T: Iterator + Discard> Discard for W<T>` did with core's
  `Filter`, whose error used to point into `<core>/iter.t`), and
  polymorphic recursion through library functions.
- Closures' symbols, as `t mono` shows them, have other numbers, as the
  libraries are read before the program.

## 0.25.0 (2026-10-23)

Step 13.1: owned strings. See [docs/13.1-string.md](docs/13.1-string.md).

### Added

- `alloc::string::String`, an owned string that grows: UTF-8 bytes in a
  `List<u8>`. `new`, `with_capacity`, `from(&str)`, `push(char)`,
  `push_str`, `pop`, `truncate` (which says `false`, and cuts nothing, if
  no character starts there), `clear`, `as_str`, `as_bytes`, `len`,
  `is_empty`, `capacity`, `from_utf8(List<u8>)` (which gives the list back
  if it isn't UTF-8), `into_bytes` and `free`; `==` and `Ord`. It isn't
  `Discard`. It's safe code in T but for one `unsafe` block, in `as_str`,
  which counts on the invariant the rest keep.

## 0.24.0 (2026-10-23)

Installing T's tree-sitter parser for syntax highlighting, from `t` itself.

### Added

- `t tree-sitter <dir>` installs T's tree-sitter parser and highlight
  queries into `<dir>/parser` and `<dir>/queries/t`, as Neovim looks for
  them, and `t tree-sitter --helix <dir>` into `<dir>/grammars` and
  `<dir>/queries/t`, as Helix does. The parser is built into `t`, so they
  match what it accepts, and installing them needs no C compiler or network.
  Files already up to date are left alone, so it's cheap to run every time.

### Changed

- Building `t` (so `cargo install t-lang`) needs a C compiler, for that
  parser.
- [editors/helix/languages.toml](editors/helix/languages.toml) has no
  `[[grammar]]` entry any more: install the parser with `t tree-sitter
  --helix` instead of `hx --grammar fetch` and `build`.

## 0.23.0 (2026-10-23)

Step 13: strings. See [docs/13-strings.md](docs/13-strings.md).

### Added

- `str`, a string: bytes that are UTF-8, only ever behind a shared
  reference, `&str`, never `&mut str`, nor what a type parameter stands
  for. `core::str` has `len`, `is_empty`, `as_bytes`, `is_char_boundary`,
  `split_at` and `get` (which cut only where a character starts),
  `chars`, and `from_utf8`, all safe code in T over two trusted
  operations: `as_bytes`, and `from_utf8_unchecked`, which is `unsafe`,
  its obligation checked as the program runs. `==` compares bytes; `Ord`
  orders them.
- `char`, a Unicode scalar value. `c as u32` never panics; `n as char`
  panics unless `n` is a scalar value, or is proved one. `char::from_u32`,
  `len_utf8`, `encode_utf8` and `is_ascii*`, in T. Character patterns and
  ranges, exhaustive over the scalar values. The refinement logic knows a
  character's number is a scalar value.
- Literals `"text"` (`&'static str`), `'a'` (`char`), `b"bytes"`
  (`&'static [u8]`) and `b'a'` (`u8`, in patterns too). Escapes `\n \r
  \t \\ \0 \' \"`; `\u{..}`, a character, only in text, and `\x..`, a
  byte, only in byte literals. `t fmt` keeps them as written.
- `==` on slices whose values are `Eq`, value by value.
- `Option::expect` and `Result::expect`.
- `print_str`, until there's I/O.
- tree-sitter: the new literals.

### Changed

- `panic` takes a message, `panic("why")`, which the panic shows.
  `panic()` is gone. `unwrap` says what it was of.
- `as` converts between integer types and `char`.

## 0.22.0 (2026-10-23)

Step 12: bits. See [docs/12-bits.md](docs/12-bits.md).

### Added

- `&`, `|` and `^` on integers: both sides and the result of one integer
  type, bit by bit on two's complement. They never panic: the result
  always fits. Not on `bool`, which has `&&`, `||` and `!=`.
- `!x` of an integer flips its bits: the type's least plus its greatest,
  minus `x`, so `!0u8` is 255 and `!0i8` is -1. Which `!` it is depends
  only on the operand's type; if that's never decided, it's "cannot
  infer".
- `x << n` is `x` times 2^n, and panics unless it fits (or is proved to);
  `x >> n` is `x` divided by 2^n, rounding down, and never panics, for any
  `n`. The same for signed and unsigned types. `n` is a `u32`.
- `&=`, `|=`, `^=`, `<<=` and `>>=`.
- Literals `0xFF`, `0b1010_0000` and `0o17`, with `_` and suffixes, in
  expressions and patterns. `t fmt` keeps a literal as it's written.
- `core::num::wrapping_add`, `wrapping_sub` and `wrapping_neg`, written in
  T for every `T: Int` and proved not to panic, and the intrinsic
  `wrapping_mul`: arithmetic modulo 2^w.
- `Signed` and `Unsigned`, built-in sealed traits with `Int` as their
  supertrait. A `T: Unsigned`'s least value is 0, and its literals may be
  0 to 255; a `T: Signed`'s least is -1 minus its greatest, and its
  literals may be -128 to 127. `core::num::abs`, `checked_abs` and
  `wrapping_abs` for `T: Signed`.
- The refinement logic knows `!x`, and `x << k` and `x >> k` for a
  constant `k`, exactly, and facts about `&`, `|`, `^` and `>>` by a
  variable: enough to prove masking and packing bytes, as UTF-8 does,
  can't panic. It knows that the greatest value of every `T: Int` is at
  least -1 minus its least.
- tree-sitter: the new operators and literals.

### Changed

- Mixing two of `&`, `|`, `^`, `<<` and `>>`, or one of them with
  arithmetic, needs parentheses: `(a & b) | c`, `(a + b) << 2`. They bind
  more tightly than comparisons, so `a & b == 0` is `(a & b) == 0`.
- `!` takes an integer as well as a `bool`, so its operand's type is no
  longer taken to be `bool` before anything says so.
- `!` of what never finishes, `!panic()`, is an error, as `-panic()`
  already was: an operator's operand must be of a type it works on, and
  `!` isn't one.

### Fixed

- The language server on Windows: it read a file's URI, `file:///C:/..`
  or `file:///c%3A/..`, as the path `/C:/..`, so it found no `main.t`
  above a module's file, and made URIs of Windows paths that editors
  didn't recognise. Open files are now looked up by path, and answered
  about by the URI the editor gave.
- The language server no longer stops at a message it can't read, such as
  one with a malformed URI: a request gets an error, and anything else is
  skipped.

## 0.21.0 (2026-10-22)

Step 11: integer types. See [docs/11-integers.md](docs/11-integers.md).

### Added

- Ten integer types: `i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`,
  `u32`, `u64` and `usize`. Each is the integers from its least to its
  greatest; `+`, `-`, `*`, `/`, `%` and unary `-` take two of one type and
  give it, and panic if the result doesn't fit (unless proved not to),
  never wrapping. Unary `-` works on unsigned types too, and fits only at 0.
- Integer literals get their type from how they're used, with no default:
  `let x = 5;` alone is "cannot infer". A suffix says it in place: `255u8`,
  `1_000_i64`. `-128` is one literal, so it fits in an `i8`. A literal must
  fit in its type: of a `usize` or `isize`, on a 32-bit machine; of a type
  parameter, in every integer type, 0 to 127.
- `x as T`: the same number as an integer type `T`. It panics if it doesn't
  fit, unless the checker proves it does, as it does for every widening
  conversion. It never truncates. A type's name follows `as`, so `x as u8 <
  y` is a comparison.
- `Int`, a built-in trait that only the integer types implement, and no
  impl can be written of: `T: Int` lets generic code use the operators,
  comparisons and literals from 0 to 127. `trait Int: Copy + Eq`.
- `core::num::least::<T>()` and `greatest::<T>()`, intrinsics whose values
  the checker knows.
- The refinement logic knows each type's range. `usize`'s greatest is a
  variable between the greatest 32- and 64-bit numbers, so what's proved
  holds on both: `u32 as usize` can't panic, `u64 as usize` may. A type
  parameter `T: Int`'s range is two such variables, at most 0 and at least
  127.
- A `match` on an integer covers its type: `0..=255` covers a `u8`, and an
  open end runs to its type's end.

### Changed

- `int` is gone. `i64` is what it was.
- Lengths and indices are `usize`: `[T]::len`, `get`, `get_mut`,
  `split_at`, `split_at_mut` and `swap`, `List::len`, `capacity`, `get`,
  `get_mut`, `swap`, `truncate` and `with_capacity`, `RawBuf::new`, `cap`,
  `grow`, `write`, `read`, `slice` and `slice_mut`, and `Iterator::count`.
- `checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_rem`
  and `checked_neg` are generic, `T: Int`; `Range<T>` is an iterator for
  every `T: Int`; `Ord` is implemented for every integer type.
- `print` takes any integer type, `print<T: Int>`, so `print(5)` alone is
  "cannot infer": `print(5i64)`.
- Messages show an integer type not known yet as `{integer}`.

## 0.20.0 (2026-10-22)

Step 10: slices. See [docs/10-slices.md](docs/10-slices.md).

### Added

- Slices: `&[T]` and `&mut [T]`, references to some number of `T`s one
  after another, with the length in the reference. `[T]` is a type only
  right after `&` or `&mut`: never a value, a field, a type argument or
  what's in a `Box`, and `*s` only a place, to borrow or call a method
  of. A type parameter never stands for a slice, so `impl<T> Tr for &T`
  isn't an impl for `&[int]`. `&[T]` is `Copy`, covariant in `T`; `&mut
  [T]` is reborrowed, invariant in `T`.
- `core::slice`: `len`, `is_empty`, `get` and `get_mut` (which return
  `None` for an index out of range), `first`, `last`, `split_at` and
  `split_at_mut` (which give a `Split` or `SplitMut` of two slices),
  `swap`, `iter` and `iter_mut`, and `slice::empty()` and `empty_mut()`.
  None is unsafe, and none panics. The intrinsics are `len`, `get`,
  `get_mut`, `split_at`, `split_at_mut` and the empty slices; the rest,
  `iter_mut` included, is safe code over them.
- `RawBuf::slice(lo, hi)` and `slice_mut(lo, hi)`, unsafe: the values in
  slots `lo..hi`, as a slice. The interpreter checks the obligation, `lo
  <= hi <= cap()` and every slot initialised.
- `List::as_slice`, `as_mut_slice` and `iter_mut`.
- `s[i]` is a syntax error that says to use `get`: indexing waits for
  refinements that can prove it in range.

### Changed

- `List::get`, `get_mut`, `iter` and `swap` are the slice's: `List::iter`
  gives a `core::slice::Iter`, and `alloc::list::Iter` is gone.
  `List::swap` takes `int`s, and says `false` for a negative one.
- `RawBuf::get` and `get_mut` are gone: `slice(i, i + 1)` does what they
  did.

## 0.19.0 (2026-10-22)

Step 9.1: lists. See [docs/09.1-list.md](docs/09.1-list.md).

### Added

- `alloc::list::List<T>`: a dense, resizable array, written in T on
  `RawBuf<T>`, with `new`, `with_capacity`, `len`, `is_empty`,
  `capacity`, `push`, `pop`, `get` and `get_mut` (which return `None` for
  an index out of range), `swap`, `iter`, and, when its values are
  `Discard`, `truncate` and `clear`. It's linear: it's used up by `free`,
  when its values are `Discard`, or by `free_with(f)`, which gives each to
  `f`. Only `push` and `with_capacity` may allocate or panic.
- `alloc::list::Iter<'a, T>`, what `List::iter` gives: a `Discard`
  iterator of `&T`s, front to back.
- `RawBuf::empty()`: a buffer of no slots, which allocates nothing, so it
  has no effects.

### Fixed

- A trait can have the name of a type in another module: `trait Range {}`
  in a program was an error, as `core` has a `Range`.

## 0.18.0 (2026-10-22)

Step 9: unsafe code and raw memory. See
[docs/09-unsafe.md](docs/09-unsafe.md).

### Added

- `unsafe` is an effect: `with unsafe` says a function has obligations
  its caller must keep, which the checker doesn't check. An `unsafe { .. }`
  block keeps those of the operations written in it, so it doesn't have
  the effect; not those of an effect parameter, nor of a closure made in
  it and called later. `main` can't say `with unsafe`.
- The `alloc` library, a package beside `core`, with `alloc::raw::RawBuf<T>`:
  an intrinsic, linear buffer of slots on the heap. `new`, `cap` and
  `grow` are safe; `write`, `read`, `get`, `get_mut` and `free` are
  unsafe, each with its obligation written down. An index's type is
  `{ i: int | i >= 0 }`, so only `i < cap()` is an obligation.
- The interpreter checks every obligation of `RawBuf`'s operations, and
  stops with "unsafe code broke its obligation", naming the operation and
  slot, and the `unsafe` block that said it was kept.
- Lints: `missing_safety_comments`, for an `unsafe` block without a
  `// SAFETY:` comment just above it, and `unused_unsafe`, for a block
  with nothing unsafe in it.
- `#[intrinsic]` on a fieldless struct, and on a method of an impl that
  isn't a trait's, in the built-in libraries.

### Changed

- The orphan rule is per package: an impl must be in the package of its
  trait or of its type's head, whichever library that is.
- `unsafe` is a keyword.

### Fixed

- `main` can't return a value that isn't `Discard`: it was thrown away at
  the end, and so leaked.

## 0.17.0 (2026-10-22)

Step 8.1: refinements on struct fields. See
[docs/08.1-field-refinements.md](docs/08.1-field-refinements.md).

### Added

- A struct's `int` and `bool` fields can have refinements, which can name
  the fields before them of the same visibility:
  `struct Span { lo: int, hi: { h: int | h >= lo } }`. Every value of the
  struct keeps them: literals and assignments to a field are checked, and
  a field a refinement is about can't be borrowed `&mut`.
- The checker knows facts about places, not just variables: `p.x`,
  `self.start`, `*r`. What a literal gives each field is known, through
  copies and patterns. Reading a field knows its struct's refinements.
- A place lent `&mut` to a call that can't keep the reference is
  forgotten after the call, rather than unknown in the whole body.
- The interpreter checks a struct's refinements at every literal,
  assignment to a field and read of one.

### Changed

- `Range::next` uses `self.start` and `self.end` as they are.
- More arithmetic is proved total, through fields, so a function or
  `main` that said `with panic` only for it gets an `unused_effects`
  warning.

## 0.16.0 (2026-10-22)

Step 8: refinement types. See
[docs/08-refinement-types.md](docs/08-refinement-types.md).

### Added

- Refinement types, `{ d: int | d > 0 }`: an `int` or `bool` with a
  predicate in linear integer arithmetic, as the whole type of a
  parameter, a result or a `let`. Arguments, results, refined `let`s and
  assignments to them are checked against them, from what's known along
  each path: refined parameters and results, conditions, `match`
  patterns, `let`s and assignments.
- An operator the checker proves can't overflow or divide by zero has no
  `panic`: `1 + 2`, `x / 2`, `x + 1` for `x < 100`.
- A `let mut` with a refinement keeps it, at every assignment: that's how
  a loop says its invariant.
- The interpreter checks every refinement at run time, and that no
  operator the checker proved panics.
- tree-sitter, `t fmt` and the LSP (hover, rename, go to definition)
  know refinements.

### Changed

- **Breaking:** `checked_add`, `checked_sub` and `checked_div` are written
  in T, proved total, rather than intrinsics; the checker trusts `swap`,
  `panic` and `checked_mul`. `Range::next` uses `+`.
- Step 7's special cases, `-5` and `/` by a non-zero literal being total,
  are now what the checker proves, along with much more: a function or
  `main` that said `with panic` only for such arithmetic gets an
  `unused_effects` warning.

### Fixed

- **Breaking:** `+=` and the other compound assignments can overflow, so
  they may panic, as their operators do. Step 7 missed them, and a
  function without `with panic` could overflow there.

## 0.15.1 (2026-10-21)

No change to the language or the tools: the type checker's two largest
files are split into modules.

### Changed

- `src/check.rs` is split into `check/expr.rs`, `patterns.rs`, `paths.rs`,
  `opaque.rs` and `adts.rs`, and `src/check/traits.rs` into
  `check/traits/`. The docs' "Code:" lines point at the new files.

## 0.15.0 (2026-10-21)

Step 7.1: impl parameters decided by bounds, and `where` on impls. See
[docs/07.1-impl-parameters.md](docs/07.1-impl-parameters.md).

### Added

- An impl's type parameter needn't be in its type if an equality bound on
  one that is decides it: `impl<U, F: Fn() -> U> Get for Thunk<F>`, or
  `impl<T, I: Iterator<Item = T>> ..`. It's what the bound says, at the
  type the impl is used at.
- `where` clauses on impls, `impl<I: Iterator> Iterator for Second<I> where
  I::Item: Discard`, and bounds on associated types in an impl's `<..>`,
  `impl<T: Tr<Out: Show>>`. A bound on what isn't part of the impl's type
  can't be of a trait that leads back to the impl's, so proving what
  implements what still ends.
- `examples/` are run by the tests, and say what they print.

### Changed

- **Breaking:** `core::iter::Map<I, F, U>` is `Map<I, F>`, and
  `core::iter::Filter<I, P, T>` is `Filter<I, P>`.
- A struct or enum with an effect parameter is an error that says so.
- The error for an impl's effect parameter that nothing decides says it's
  an effect parameter, and how to decide it.
- `t fmt` breaks a long impl header in its `<..>`, keeping a short type
  after `for` on one line, rather than in the type.

## 0.14.0 (2026-10-21)

Step 7: effects. See [docs/07-effects.md](docs/07-effects.md).

### Added

- A function's type says what a call may do besides give its result: its
  effects. `with panic` says it may abort the program, and `with alloc`
  that it may allocate; `with {alloc, panic}` says both. A function
  without a `with` can do neither, which is checked: nothing in it, and
  nothing it calls, can. A `main` without one is a program that can't
  panic.
- What panics: `+`, `-`, `*` and unary `-` on overflow, `/` and `%` by
  zero (but not `-5`, or `/` and `%` by a literal other than `0`, which
  can't), `Box::new`, which also allocates, and the new `panic()`, of
  type `fn() -> ! with panic`.
- Effect parameters, lowercase among the type parameters, for code
  generic in what it's given: `fn twice<e, F: Fn(int) -> int with e>(f: F,
  x: int) -> int with e`. Each call's is worked out as the least that
  will do, the effects of what's passed.
- A closure's effects are worked out from its body. A bound like `F:
  Fn(int) -> int with panic` says the most a closure may do.
- Associated effects: `effect e;` in a trait, which each impl says, as it
  says an associated type. `Iterator` has one, `e`, which its `next`
  has: iterating over a range can't panic, and `map` and `filter` have
  the effects of what they iterate and what they call.
- In the core library: `Option::unwrap` and `Result::unwrap`, which panic,
  and `checked_add`, `checked_sub`, `checked_mul`, `checked_div`,
  `checked_rem` and `checked_neg`, which can't, in the prelude.
- A warning, `unused_effects`, for an effect a private function says but
  can't have, with a fix that takes it out. Not for a `pub` function,
  which may keep the right to have it later, nor a trait's methods or
  their impls'.
- The interpreter checks, at each panic and allocation, that every
  function being run says it may: a "type soundness violated" error if
  not, as for leaks.

### Changed

- Breaking: every function that may panic or allocate must say so, `main`
  too, and every `Fn` bound or function type that doesn't say `with`
  holds only of what can't. A type parameter named in lowercase is now
  an effect parameter.
- An impl of a trait with an effect parameter that isn't in its type is
  allowed if an `Fn` bound decides it, as `impl<f, F: Fn() with f> Tr for
  W<F>`.
- A function's `with` and `where` go on lines of their own together when
  its signature doesn't fit on one.

- Internal: every expression, pattern, name, type and closure in the AST
  has an id of its own (`ast::NodeId`), and what the checker learns about
  a node is filed by that, not by its span or its address in memory. Nodes
  made up by rewriting `for` loops and ranges no longer need spans nothing
  else has, and a closure's result lifetimes still to work out are a
  region of their own, `Region::Result`, rather than a name. Programs mean
  what they did.

### Fixed

- Generic code runs as it was checked, whatever its type parameters turn
  out to be. A field of a type parameter not known to be `Copy` that a
  method moved out of, as in `app(self.p)`, used up `self` when checked,
  but was copied when run at a `Copy` type, such as `int` or a closure that
  only borrows, and then `self`, if it isn't `Discard`, was thrown away: a
  leak, which the interpreter stopped at. Now what the borrow checker
  moved is moved at every type, and a pattern uses up, at every type, what
  it took apart that wasn't known to be `Discard`.
- On Windows, the file of a module in a directory, as `a/b.t` for `mod b;`
  in `a.t`, is shown with Windows' separators, `a\b.t`, rather than a mix.

## 0.13.0 (2026-10-20)

Step 6e.1: higher-ranked closures. See
[docs/06e.1-higher-ranked-closures.md](docs/06e.1-higher-ranked-closures.md).

### Added

- A closure's type can be generic over lifetimes, as a function type is:
  over those a bound like `F: Fn(&int) -> bool` says, where one is wanted,
  or else over the lifetimes left out in its parameters' annotations.
  `let id = |x: &int| x;` can be called with references that live for
  different lengths of time, and a closure made before an impl's bound is
  asked meets it.
- A closure's result's lifetimes, written or not, are worked out from its
  body, unless a bound says them: the lifetime of the one parameter it
  returns a reference from, or else one fixed where it's made, such as of
  what it captured. `|x: &int| x` returns a reference as long-lived as `x`;
  `|_x: &int| -> &int { &n }` one as long-lived as its borrow of `n`. A
  result that could borrow from either of two parameters is an error.
- `Iterator::filter`, and `core::iter::Filter`. It throws away what the
  predicate doesn't keep, so it asks for `Self::Item: Discard`.

### Changed

- Errors in a closure about lifetimes a call picks say which parameter's,
  and where it went: into what the closure captured, or its result.
- What a type with a closure in it implements, where the closure's result's
  lifetimes are worked out, is decided at the end of the function, after
  its captures and calls.

## 0.12.0 (2026-10-20)

Step 6e: a small core library. See [docs/06e-core.md](docs/06e-core.md).

### Added

- The core library, `core/`, written in T and built into the compiler: every
  program has it. `core::option::Option` and `core::result::Result`, with
  `is_some`, `as_ref`, `take`, `map`, `and_then`, `or_else`, `unwrap_or`,
  `unwrap_or_else`, `ok` and the like;
  `core::cmp::Ord` and `Ordering`, for `int` and `bool`; `core::iter::Iterator`
  with `map`, `fold` and `count`; `core::ops::Range`; and `core::mem::swap`
  and `replace`. A method that would throw a value away asks for `Discard`
  on the method, so an `Option` of a linear value has the rest.
- The prelude: `Option`, `Result`, `Ord` and `Iterator` are in scope in every
  module, after what the module declares and imports itself, so a program's
  own `Option` still wins.
- Paths can start with `core` anywhere. No module can be called `core`.
- `for p in it { .. }` over any `Iterator`, with labels, `break` and
  `continue`. It means a `loop` that calls `next`, so its iterator must be
  `Discard` (iterate a linear one through `&mut it`), and each item must be
  used up by the body. A `break` in it can't give a value.
- `a..b`: a `core::ops::Range`, from `a` up to `b`, empty if `a` isn't before
  `b`. `Range<int>` is an iterator, which never overflows. Its fields are
  private. The `empty_ranges` lint warns about one like `5..0`.
- `where` clauses after a function's signature, which can bound `Self` and
  associated types (`where Self: Discard, Self::Item: Eq`). An impl's
  method has its trait's exactly.
- `#[intrinsic]`: a function the core library declares with no body, which
  the compiler provides. The only one is `core::mem::swap`.
- The orphan rule between packages: an impl must be in the package of its
  trait or of its type's head, and the built-in traits and types are the
  core library's.
- The formatter, the tree-sitter grammar and the language server know `for`,
  ranges, `where` clauses and `#[intrinsic]`. Hovering a core library item
  shows its documentation; it has no definition to go to, and can't be
  renamed.

### Changed

- `replace` and `swap` aren't built in any more: they're
  `core::mem::replace` and `core::mem::swap`, and need a `use` or the path.
  Using one without either says where it is, with a fix that imports it.
- `in` is a keyword.
- An `impl` bound like `P: FnMut(I::Item) -> U` can name associated types of
  the impl's other parameters.

### Fixed

- A method called by path, `Tr::m(x)`, has its trait proved once its
  arguments say what `Self` is, rather than failing later on a projection.

## 0.11.0 (2026-10-19)

Step 6d.1: modules in files of their own. See
[docs/06d.1-files.md](docs/06d.1-files.md).

### Added

- `mod m;`: a module whose items are in a file of its own. It means just
  what `mod m { .. }` would with the file's items in the braces.
- The file of module `a::b` is `a/b.t`, from the directory of `main.t`,
  whether `b` is `mod b;` in `a.t` or in an inline `mod a { .. }`.
- Only a program that starts in `main.t` has modules in files, and none of
  them can be called `main`, so a file's program is the nearest `main.t`
  above it that has it. A missing file, and a file read twice through a
  link, are errors.
- Errors and warnings say which file they're in; fixes edit each file they
  touch; an `#[allow(..)]` on `mod m;` covers `m.t` and the files of the
  modules in it.
- `t run`, `t check` and `t mono` read a program's files. `t fmt` formats
  `mod m;`, and formats each file it's given on its own.
- The language server checks a file as part of its program, using the
  editor's text for open files. Diagnostics go to the file they're in, and
  going to a definition, references and renames cross files; going to the
  definition of `m` in `mod m;` opens `m.t`.
- The tree-sitter grammar knows `mod m;`.

## 0.10.0 (2026-10-19)

Step 6d: modules. See [docs/06d-modules.md](docs/06d-modules.md).

### Added

- `mod m { .. }`: a module, a namespace of its own, holding items as the
  file does. Each module has a namespace of types (structs, enums, traits
  and modules) and one of functions. Nothing of the modules around it is
  in scope in it.
- Paths to items: `a::b::f`, and from `crate::`, `super::` and `self::`, in
  expressions, types, patterns and bounds.
- `use a::b;`, `use a::b as c;`, `use a::{b, c};` and `pub use`, in a
  module. Two names for the same thing in a namespace of a module, and
  `use`s that go round in a circle, are errors.
- `pub` on items, fields and the methods of inherent impls: an item is
  visible in its module and the modules in it, and, if it's `pub`,
  wherever its module is. An enum's variants are public wherever it is.
- A struct with fields that can't be seen is abstract: it can't be made
  there, nor, unless it's `Discard`, taken apart, by moving a field out or
  by a pattern that matches it by value.
- A public item can't name a private type or trait in its signature, nor a
  public field in its type.
- `x.m()` only finds the methods of traits in scope where it's written,
  with a fix that imports the one that has it.
- `#[derive(Discard where T: Discard)]`: a public struct with private
  fields says what each derive needs, which must be enough for its fields.
- An inherent impl must be in its type's module, and `main` at the crate
  root.
- The `unused_imports` lint, with a fix that removes the `use`.
- The tree-sitter grammar, the formatter and the language server know
  modules, paths, `use` and `pub`; renaming follows names through paths and
  `use`s, and the outline nests modules.

### Changed

- `pub`, `mod`, `use`, `crate`, `super` and `where` are keywords.

## 0.9.0 (2026-10-19)

Step 6c.1: returning closures, opaque types. See
[docs/06c.1-opaque-types.md](docs/06c.1-opaque-types.md).

### Added

- `impl Fn(int) -> int + Discard`, anywhere in the return type of a
  function, nested function or method of an inherent impl: an opaque type,
  standing for the one type the function's body returns there. Any bounds
  can follow `impl`, like `impl Iter<Item = int>`. Callers know only what
  they say, `Discard` and `Copy` included, and the impls that apply to any
  type that does; each `impl` written is a type of its own.
- An opaque type may borrow for any lifetime of its function, elided ones
  too, and hold any of its type parameters, and is invariant in them. The
  borrow checker makes each region of the type behind it one of those
  lifetimes, or `'static` (member constraints), and checks its bounds with
  their lifetimes.
- A type behind an opaque type that holds that opaque type, through others
  or not, is an error.
- The tree-sitter grammar, the formatter and the language server know
  `impl` types.

### Changed

- `t mono` writes the type behind each opaque type, as `{closure#12}` for a
  closure's.

## 0.8.0 (2026-10-19)

Step 6c: closures as `Fn` traits. See
[docs/06c-closures.md](docs/06c-closures.md).

### Added

- Each closure that captures has a type of its own, `{closure}`: a struct of
  what it captures, generic over the type parameters in scope, and invariant
  in its lifetimes.
- The built-in traits `Fn`, `FnMut` and `FnOnce` (`Fn: FnMut: FnOnce`), as
  bounds like `F: Fn(int) -> int`, with the signature higher-ranked over any
  lifetime it leaves out. Functions implement all three; `&F` is `Fn` when
  `F` is, and `&mut F` is `FnMut` when `F` is. Nothing else can implement
  them.
- Closures capture each variable by `&`, by `&mut` or by value, the least
  that does what the body does with it; `move |x| ..` captures everything
  by value. A `&mut` the body changes things through is borrowed uniquely,
  which doesn't need it to be `mut`. A closure is `FnOnce` only if it
  moves out of what it owns, or owns something that isn't `Discard` (which
  its body must then use up), `FnMut` if it changes what it captured, and
  `Fn` otherwise.
- A closure is `Discard` and `Copy` if what it captured is.
- `move` is a keyword.
- The tree-sitter grammar and the formatter know `move` and `Fn` bounds.

### Changed

- Closures that capture can be stored, and passed to generic code, as long
  as they don't outlive what they borrow: the downward-only rule is gone.
  They can't be returned from a function yet, as their types can't be
  written (step 6c.1).
- A function type holds only functions and closures that capture nothing,
  so a function that takes a closure that captures is generic over it.
- Captured variables are borrowed, not copied: a closure sees changes made
  after it's made, and can change them, and what it borrows can't be
  changed or moved while it may still be called.
- A borrow through a `&` may last as long as that reference says, whatever
  happens to the way it was reached (Rust's supporting prefixes).
- `t mono` writes calls of closures with the borrow they take,
  `(&mut f)(1)`.

### Removed

- Monomorphisation no longer checks the program it makes again, and the
  checker's mode for doing so is gone: no error depends on a copy of a
  generic function any more.

### Fixed

- An equality bound naming a lifetime, like `T: Tr<Out = &'a int>`,
  crashed the borrow checker, and its regions weren't related where the
  function was called, which allowed a use after free.

## 0.7.0 (2026-10-18)

Step 6b: associated types. See
[docs/06b-associated-types.md](docs/06b-associated-types.md).

### Added

- Associated types: `type Item;` or `type Item: Discard + Show;` in a trait,
  `type Item = int;` in each impl of it. An impl must say every one, and
  only those; what the trait's bounds say of it is checked at each impl.
  There are no defaults.
- Projections: `<T as Tr>::Item`, and `T::Item` or `Self::Item` for a type
  parameter or `Self`, when exactly one of its traits has an `Item`. On a
  known type, a projection is what the impl says, with the type's lifetimes
  where the impl's header has its own; on a type parameter it's a type of its
  own, equal only to itself.
- Equality bounds, `T: Tr<Item = int>`, on functions, impls and structs, and
  bounds on projections, `T: Tr<Item: Discard>`, on functions. Each use
  shows they hold.
- `type` and `as` are keywords.
- The naming lint covers associated types, which are named like types.
- The tree-sitter grammar and the formatter know associated types,
  projections and the new bounds.

### Changed

- What an impl says an associated type is can only mention the impl's
  parameters that are in its type outside function types, and `'static`,
  and leaves no lifetime out. A projection in it must be on a smaller type
  than the impl's, or, in a blanket impl, on its parameter, with no cycle
  between traits.
- An impl can't be for a projection.
- A struct or enum with a field of a projection's type is invariant in the
  parameters the projection mentions.
- A rigid projection outlives what its type does, or what the signature
  says it does (`T: 'r` gives `T::Item: 'r`).
- Inference works a projection out once its type is known, and never infers
  the type from what the projection is.
- Polymorphic recursion through a projection is rejected, in functions, in
  method calls on a projection, and in the types impls' associated types
  mention; so are types that hold themselves through one.

## 0.6.0 (2026-10-18)

Step 6a.1: linear types. See
[docs/06a.1-linear-types.md](docs/06a.1-linear-types.md).

### Added

- A built-in trait `Discard`: values of its types may be thrown away.
  `#[derive(Discard)]` and `impl Discard for P {}` say a struct or enum is,
  and need every field to be; a derived impl needs of the type parameters
  only what the fields do. `int`, `bool`, `()`, `!`, functions and
  references always are, and `Box<T>` is when `T` is.
- `replace(&mut x, v)`, built in: puts `v` in `x` and gives back what was
  there, for any type.
- `swap(&mut a, &mut b)`, built in: exchanges two values, for any type.

### Changed

- Values are linear: one whose type isn't `Discard` must be used up, moved
  or taken apart by a pattern, on every path that goes on. Letting it go
  out of scope, ignoring it as a result, binding it to `_`, or giving its
  place a new value (`x = v`, `*r = v`) is an error that says how to fix
  it. Paths that never go on, after a call that returns `!`, are exempt.
- Matching a value that isn't `Discard` by value against a struct or
  variant pattern uses it up, in every arm; the parts the pattern doesn't
  bind must be `Discard`.
- `trait Copy: Discard`: `#[derive(Copy)]` needs `#[derive(Discard)]` too,
  and the error's fix adds it.
- A `T` isn't `Discard` unless a bound says so: generic code that drops a
  `T` needs `T: Discard`, or `T: Copy`.
- An unused variable whose type isn't `Discard` is warned about with "use
  it up" rather than a fix to rename it `_x`, which wouldn't help.
- Every struct and enum in the tests, docs and examples is
  `#[derive(Discard)]` now.
- The interpreter moves every value whose type isn't `Copy`, not only
  those holding a box, and stops with "type soundness violated" where a
  value that isn't `Discard` is thrown away, but on the way out of a panic.

A soundness pass over traits, linear types, the call rule and `swap`, each
attempt a test (`tests/t/*soundness_*.t`).

### Fixed

- A borrow passed to a call worked out what it borrows from (`make()` in
  `&*make()`) after the other arguments in the interpreter, but in order in
  the borrow checker, which so checked a different program: one where an
  argument after it could move what it then used.
- A coerced `&mut x` argument, used as a `&T`, was related to its
  parameter's type part by part, skipping the parts that didn't line up,
  so a reference out of it could outlive what it pointed to. Coerced
  arguments are now borrows taken after the arguments, as `lift` writes
  them (`&**r`), and relating two types that aren't the same up to regions
  is an internal error.
- `==` on a type with an impl of `Eq` of its own compared field by field,
  unless something else called that `eq` by name. And `==` was invisible
  to the polymorphic recursion check, so an `eq` comparing at a bigger type
  each time made monomorphisation go on forever; it now sees `==` as the
  call of `eq` it is, through references, boxes and derived impls.
- A derived impl's bounds came out as the derived trait even where a
  field's impl needed another (`Q<T>: Eq` needing `T: Show`), so a
  `#[derive(Discard)]` type could be thrown away holding a value that
  isn't, and a derived `Eq` crashed monomorphisation.
- "moved here, in an earlier time round the loop" was said of a move
  written after a borrow that waits for the arguments, with no loop.

Step 6a: traits. See [docs/06-traits.md](docs/06-traits.md).

### Added

- `trait` declarations, with methods, default methods and supertraits
  (`trait Ord: Eq`), and `impl Trait for Type`, generic ones too
  (`impl<T: Show> Show for Option<T>`), for any type.
- Bounds on type parameters of functions, structs, enums and impls:
  `T: Show`, `T: Show + Eq + 'a`. Every use must show they hold; none are
  implied.
- Inherent impls, `impl Point { .. }`, for structs and enums, with methods
  that take `self`, `mut self`, `&self` or `&mut self`, and ones that take
  none. `Self` is the impl's type in it.
- Method calls `x.m(..)`, and `x.m::<int>(..)` for a method's own type
  parameters. They look through references and boxes to what `x` is, and
  take `&` or `&mut` of that if the method's receiver wants it. Inherent
  methods come before traits'; two traits with a method of the name are
  ambiguous.
- Paths to methods: `Point::new(1, 2)`, `Show::show(&x)`, `T::m(..)`, and
  `Point::len` without `(..)` as a function value.
- A call takes its borrows of places (`&v`, `&mut v`, and a method
  receiver's) after its other arguments, so `v.push(v.len())` and
  `f(&mut v, v.len())` work, and so does `v.push({ v.clear(); 1 })`.
- `Eq` is a trait, behind `==` and `!=`, and `#[derive(Eq)]` compares field
  by field. An impl of `Eq` can compare as it likes, and a derived one uses
  it for that field.
- `Copy` is a trait; `#[derive(Copy)]` is an impl of it, and so is
  `impl Copy for P {}`. `T: Copy` lets generic code copy a `T`. A derived
  impl needs of the type parameters only what the fields do.
- Coherence: no two impls of a trait for types that could be the same. And
  resolution always ends: impls need things of smaller types, or, for
  blanket impls like `impl<T: A> B for T`, of other traits with no way back.
- A trait impl holds at every lifetime: each lifetime and type parameter in
  its type appears once, with no `'static` and no lifetime bounds, since
  resolution doesn't look at lifetimes.
- The polymorphic recursion check follows calls of traits' methods to the
  impls they may run.
- Lint warnings for methods: `dead_code` for inherent methods nothing uses,
  and the naming lints for traits and methods. No `unused_variables` for an
  unused `self`.
- `t lsp`: hover, go to definition, references and rename for traits and
  methods (renaming a trait's method renames its impls'), and traits and
  impls with their methods in the outline.
- The tree-sitter grammar knows traits, impls, receivers and method calls.

### Changed

- `==` on a struct or enum needs it to implement `Eq`: add
  `#[derive(Eq)]`. `==` on a `T` needs a bound `T: Eq`.
- `s.f(x)` is a call of method `f`. To call a function held in field `f`,
  write `(s.f)(x)`; the error says so.
- `trait`, `impl` and `for` are keywords, and `self` can only be a method's
  receiver.

## 0.5.1 (2026-10-17)

Step 5.1: control flow, and the type with no values. See
[docs/05.1-control-flow.md](docs/05.1-control-flow.md).

### Added

- `return e`, and `return` for `return ()`, from a function or closure. A
  closure's result type is worked out from its `return`s as well as its
  body.
- `loop { .. }`, which runs until a `break`. `break e` gives it the value
  of `e`, and `break` gives it `()`; with no `break` that gets out, it never
  ends.
- `break` and `continue`, in a `loop` or `while`. A `while` loop's `break`
  can't have a value, since the loop can also end without one. Neither can
  get out of a closure.
- `!`, the type with no values, said *never*. `return`, `break` and
  `continue` are of type `!`, as is a `loop` that never ends and a `match`
  with no arms, and a `!` can be used as any type, as it never gives a
  value: `None => return -1` fits as an arm of an `int` `match`. It can be
  written, as in `fn forever() -> ! { loop {} }`, and anywhere else a type
  can: an `Option<!>` is never `Some`, so a `match` needn't cover it.
- A block that never gets to its end, say as it ends with a `return`, is a
  `!`, so a function can end with `return x;`.
- A warning, `unreachable_code`, for code after something that never
  finishes, like a `return`, as rustc gives.
- The borrow checker follows jumps: a `return`, `break` or `continue` ends
  the scopes it leaves, so a box moved just before a `break` is fine. Dead
  code isn't borrow checked.
- The interpreter frees what a jump leaves, the parts of an expression it
  had worked out included, as in `f(Box::new(1), return 2)`.
- Labels on loops: `'outer: loop { .. }` and `'outer: while c { .. }`,
  and `break 'outer`, `break 'outer x` and `continue 'outer` to leave or
  go round a loop around the innermost. A loop can't have the label of
  one around it. See
  [docs/05.1-control-flow.md](docs/05.1-control-flow.md#labels).
- A warning, `unused_labels`, for a label no `break` or `continue` names.
  Its fix removes it.
- Syntax highlighting for all of the above.

### Changed

- `return`, `break`, `continue` and `loop` are keywords.
- `let x = match n {};` makes `x` a `!`, rather than being an error for
  not saying what type it is.

## 0.5.0 (2026-10-16)

Step 5: ownership, borrowing and pointers. See
[docs/05-ownership.md](docs/05-ownership.md).

### Added

- References: `&x` and `&mut x`, of types `&T` and `&mut T`, and `*r` for
  what `r` points to. A field of a reference is a field of what it points
  to (`r.x`), and a `&mut T` can be used as a `&T`, and a `&Box<T>` or
  `&&T` as a `&T` (deref coercion).
- Lifetimes: `&'a T`, `fn f<'a>(..)`, `struct S<'a>`, bounds like
  `'b: 'a` and `T: 'a`, and Rust's lifetime elision for signatures. A
  struct or enum can hold a reference, so a type can hold itself through
  one. A lifetime left out in a function type is one each call picks:
  `fn(&int) -> int` takes references of any lifetime.
- `Box<T>`, a value on the heap: `Box::new(x)` allocates, `*b` is what it
  holds, and it's freed when its owner goes away. A type can hold itself
  through a box: `enum List { Nil, Cons(int, Box<List>) }`.
- Mutation: `let mut`, `mut` parameters and pattern bindings, assignment to
  variables, fields and `*r`, and `+=`, `-=`, `*=`, `/=` and `%=`.
- `while` loops.
- A reference or box to something with no values, like `&Never`, has
  none either, and nor does a type that must hold itself forever, like
  `struct Stream { next: Box<Stream> }`: a `match` needn't cover them.
  Rust asks for those arms, since unsafe code there may make a `&Never`.
- A warning, `infinite_types`, for a struct or enum that holds itself with
  no way to stop, like `struct Stream { next: Box<Stream> }`, or two that
  hold each other: it has no values.
- `let x;`: a variable declared with no value, given one later by
  assignment. It can't be used before then, and it can be assigned once
  without being `mut`.
- Moves: using a value whose type isn't `Copy` moves it, and it can't be
  used again until it gets a new value. `int`, `bool`, `()`, functions and
  shared references are `Copy`, and so are structs and enums with
  `#[derive(Copy)]`, if their fields are. `#[derive(..)]` is the second
  attribute; `Copy` is all it derives.
- The borrow checker (`src/borrowck`), as in rustc since 2018: non-lexical
  lifetimes, worked out on a lowered form of each function (MIR). It
  rejects using a value after it's moved, moving out from behind a
  reference, changing what isn't `mut` or is behind a `&`, using or
  changing something in a way that clashes with a borrow of it, references
  that outlive what they point to, and functions that need more of their
  lifetimes or type parameters than their signatures say.
- Matching on a reference looks through it, and binds variables by
  reference (match ergonomics).
- A warning, `unused_mut`, for a variable declared `mut` that's never
  changed. Its fix removes the `mut`.
- The interpreter keeps memory the way a compiled program would, and
  checks for dangling references, uses after moves, double frees and leaks
  as it runs.
- Errors can have notes, pointing at other places that explain them:
  "`x` is borrowed here".
- Syntax highlighting for all of the above.

### Changed

- `Box` is a built-in type, so a struct, enum or type parameter can't be
  called that.
- Closures can only capture values that are `Copy` and hold no
  references, since what they capture is copied into them, and can't
  change what they capture. Assigning a closure that captures, or a
  function read through a reference, is an error, as returning and storing
  one are.
- A struct or enum that contains itself suggests holding itself through a
  pointer, like `Box<List>`, instead.

## 0.4.5 (2026-10-15)

### Added

- `t --help` (or `-h`) prints what each command does, on stdout, and `t
  --version` (or `-V`) prints the version. Both used to print the usage as
  if they were a mistake, and exit with 2.

## 0.4.4 (2026-10-15)

### Added

- `#[allow(..)]` turns lints off: before a `fn`, `struct`, `enum`, field or
  variant, top-level or nested, it turns off the lints it names in all of
  it, as in Rust. It goes before or after the doc comment; `t fmt` puts it
  after. A name that isn't a lint's is a new warning, `unknown_lints`, with
  a fix when it's close to one (`dead_cod` → `dead_code`). `#[allow]` is the
  only attribute; any other is a syntax error, as is one before anything
  else, like a `let`.
- `t check` and `t run` name a lint the first time it warns, with how to
  turn it off: "= note: `#[allow(dead_code)]` turns this off". `t lsp`
  already gave the lint as each warning's code.
- Syntax highlighting for attributes.

### Changed

- Syntax highlighting: the `x` of a shorthand `P { x }`, in a struct
  literal or pattern, is highlighted as the variable it is rather than as
  a field, as `t lsp` takes it for hover and rename.

## 0.4.3 (2026-10-15)

### Added

- Naming lints, as in Rust: `non_camel_case_types` for a struct, enum,
  variant or type parameter whose name isn't in `UpperCamelCase`, and
  `non_snake_case` for a function, parameter, variable or field whose name
  isn't in `snake_case`. Leading underscores don't count and digits are
  fine (`_unused`, `Vec2`, `x1`), and an acronym is one word, so
  `HTTPServer` should be `HttpServer`, and `IO` should be `Io`: stricter
  than rustc, as Rust's style guide asks, so all capitals are left for
  globals. Each is reported once, where the name is declared, as rustc
  words it: "type `my_point` should have an upper camel case name:
  `MyPoint`". The fix renames it everywhere, as `t lsp`'s rename does, so
  it's offered only when the program checks and the new name isn't a
  keyword and clashes with nothing. The test programs that broke the rules
  are renamed.

- Nested functions: a `fn` can be declared as a statement in any block. It
  means exactly what it would at the top level; only its name is scoped, to
  the whole block, so it can be used before it's declared, be recursive, and
  call the other functions of its block. It can be generic and have a doc
  comment, and it shadows any outer function of the same name. It can't
  capture: using a local of a function it's in is an error, "can't capture
  `x` in a nested `fn`; use a closure", whose fix makes it a closure when
  that works. See docs/03-inference.md.
- Local types: a `struct` or `enum` can be declared in a block too, by the
  same rules. Its name is in scope in the whole block and shadows any outer
  type of the same name, and its values can leave the block by inference.
  It can't use the type parameters of the function it's in. When two
  different types of the same name meet in a type error, the message says
  so, and where each is declared.
- The checker resolves each use of a function or type to its definition,
  through the scopes it's in, rather than by name. Once checked, nested
  functions and local types are moved to the top level under names of
  their own (`main::helper`, `main::Point`), which `t mono` shows, so
  monomorphisation and the interpreter never see one. Messages, printed
  values, `t lsp`'s hover and go to definition show and find them as
  written, even when several share a name.
- "Never used" warnings for nested functions that nothing in their block
  uses, and the dead-code warnings for structs, enums and variants cover
  local ones.
- `t lsp` finds references, highlights every place something is named
  (its declaration as written, its uses as read), and renames: variables
  and parameters (of closures and patterns too, every alternative of an
  or-pattern), functions, structs, enums, variants, fields and type
  parameters, top-level or local. All of these follow what each name
  refers to, not how it's spelled, through the same query
  (`Info::occurrences`). Renaming either side of a shorthand `P { x }`
  spells out the other. A rename is refused if the new name isn't an
  identifier, is a keyword, or is `int` or `bool` for a type, and if it
  would change what any name refers to: the renamed program is checked
  again, and the message says what it would clash with. Built-ins and
  `main` can't be renamed, and a file that doesn't check can't be renamed
  in.
- `t lsp` goes to type definitions, from a variable, parameter, field or
  function to each struct and enum in its type, and gives an outline of
  the file (document symbols): functions, structs and enums, with fields
  and variants under their type and nested functions and local types
  under their function, each with its signature. The outline comes from
  the parse alone, so it's there while the file has a type error.
- In a shorthand `P { x }`, hover, go to definition and the rest take
  `x` to be the variable rather than the field, as renaming does. The use
  of a function with type arguments, like `id::<int>`, is just `id` to
  them.
- `t fmt` formats nested functions and local types like top-level ones,
  with their doc comments, and keeps blank lines around them as around any
  statement. The tree-sitter grammar parses them.

## 0.4.2 (2026-10-15)

### Added

- Doc comments: `///` before a `fn`, `struct`, `enum`, struct field or enum
  variant documents it, in Markdown. `t lsp` shows it on hover, after the
  signature, on the declaration and on every use. Doc comments are part of
  the AST, so `t fmt` prints them from there and its check that the output
  is the same program covers them. `////` is a plain comment, as in Rust.
- A warning, `unused_doc_comments`, for a `///` that documents nothing, such
  as one before a statement. Its fix makes it a plain `//` comment.
- The tree-sitter grammar tells doc comments from plain comments, and the
  highlight queries capture them as `@comment.documentation`.
- `cargo test` checks the T code in the README and `docs/`: every
  ```` ```rust ```` block must parse and be formatted, and a whole program
  must run and print what its `// expect:` comments say, as in `tests/t`.
  Fragments and deliberate errors are marked in the fence
  (```` ```rust,fragment ````, ```` ```rust,error ````,
  ```` ```rust,unformatted ````); see the README's Tests section.

### Fixed

- Docs whose code no longer worked: step 3 opened with an `adder` that
  returns a closure capturing `n`, an error since 0.4.0, and step 4's
  opening program left a variable unused. Several examples weren't
  formatted, and some mixed items with statements, which doesn't parse.

## 0.4.1 (2026-10-15)

### Added

- `t fmt <file>`, a formatter with one style and no options: 100 columns,
  four-space indentation, a line broken only where it doesn't fit (with
  Wadler's "prettier printer" algorithm). It keeps comments, and single
  blank lines between items, statements and arms, and lines up comments at
  the ends of consecutive lines. `t fmt --check` prints a diff and fails if
  a file isn't formatted. It parses its own output before writing, and
  refuses if that's a different program. See [docs/fmt.md](docs/fmt.md).
- `t lsp` formats documents, so editors can format on request or on save.
  See the README for Neovim and Helix; the Helix config formats on save.

### Changed

- `t mono` prints in `t fmt`'s style: short declarations and blocks that
  are one expression go on one line, and long lines break.
- Building T needs Rust 1.99 or later, up from 1.88. T is an application, so
  it tracks the Rust it's built with rather than holding to an old one.

### Fixed

- `t mono` printed a range pattern like `0..1` as `0`, and `(a < b) < c`
  without its parentheses, which doesn't parse.

## 0.4.0 (2026-10-15)

Step 4: generics. See [docs/04-generics.md](docs/04-generics.md).

### Added

- Generic functions, structs and enums: `fn id<T>(x: T) -> T`,
  `struct Pair<A, B> { .. }`, `enum Option<T> { Some(T), None }`, and
  types like `Option<int>`. Type arguments are inferred at each use, or
  written out with a turbofish: `id::<int>(1)`, `Option::<int>::None`, with
  `_` for parts to infer (`second::<bool, _>(..)`). Inside
  a generic body a type parameter is opaque, and `==` on one is an error
  until traits. Closures and `let`s stay monomorphic.
- Monomorphisation, as a pass between checking and running: each generic
  function and type is copied once per list of type arguments it's used
  with. `t mono <file>` prints the result. Polymorphic recursion that would
  need infinitely many copies is an error. Each copy is checked again, so
  using a generic that returns or stores its argument at a function type
  (`id(|x| x + n)`) is rejected like the non-generic version would be.
- `t lsp` answers hover, go to definition and inlay hints (inferred types,
  and the type arguments of generic uses).
- A tree-sitter grammar with highlight queries, in `editors/tree-sitter-t`,
  and setup notes for syntax highlighting in Helix and Neovim.

### Changed

- Closures that capture variables are downward-only: they can be called and
  passed to functions, but not returned or stored in a struct or enum, since
  the variables they capture live on the stack. `fn adder(n: int) ->
  fn(int) -> int { |x| x + n }` is now an error, and so is returning a
  function-typed parameter. Closures that capture nothing, and top-level
  functions, go anywhere as before. This keeps closures compilable without a
  heap or a garbage collector.
- The roadmap is reordered around compiling to bare metal: generics, then
  ownership (with types that hold themselves through pointers, in place of
  a separate recursive-types step), traits, effects, and refinement types.

## 0.3.0 (2026-10-14)

Step 3: type inference and closures. See
[docs/03-inference.md](docs/03-inference.md).

### Added

- Closures: `|x| x + 1`, `|x: int, y| -> int { x * y }`, and `|| 1` with no
  parameters. They can use the locals around them, be passed, returned and
  stored in structs, and have the same `fn(..) -> ..` types as top-level
  functions.
- Type inference inside function bodies. A closure parameter's type comes
  from its annotation, from the type expected where the closure is written
  (`apply(|p| p.x, ..)`), or else from how it's used, by unification. A
  variable whose type is never worked out is an error asking for an
  annotation, as is one that would need an infinite type (`|f| f(f)`).
- `let x = match n {};` with no arms and no annotation takes the type `x` is
  used at, instead of being an error.

## 0.2.3 (2026-10-14)

### Added

- A warning for code that can never run (`dead_code`): functions `main`
  doesn't reach, even if they call each other, structs `main` never builds,
  enums it never mentions, and variants it never constructs. Names starting
  with `_` are exempt. Editors fade it out.

## 0.2.2 (2026-10-14)

### Added

- Warnings. Unlike errors they don't stop the program; `t` prints them to
  stderr and runs it anyway.
- Fixes for warnings and some errors, shown as `help:` lines and offered by
  `t lsp` as quick fixes (code actions): remove an unreachable arm or
  alternative, narrow or merge a range, add the missing `;` after an `if` or
  `match` statement (or wrap it in parentheses), and use `|` instead of `||`
  between pattern alternatives.
- `t lsp` reports every warning, not just the first problem. Each warning has
  a code naming its kind (`unreachable_patterns`, `overlapping_ranges`,
  `mergeable_ranges`), and unreachable patterns are tagged as unnecessary, so
  editors fade them out.
- A warning for variables nothing reads (`unused_variables`), with a fix
  that renames `x` to `_x`, everywhere an or-pattern binds it, and
  `P { x }` to `P { x: _x }`. Names starting with `_` are exempt.

### Changed

- Unreachable arms and alternatives, ranges that can be narrowed, and ranges
  that can be merged are warnings instead of errors, and every one is
  reported instead of just the first.

## 0.2.1 (2026-10-14)

### Added

- Range patterns on ints: `1..10` (excluding 10), `1..=9`, `10..`, `..0` and
  `..=-1`. Exhaustiveness checking understands them, so
  `match n { ..0 => a, 0.. => b }` needs no `_`, and an empty range like
  `5..5` is an error.
- An error for a range whose end the patterns before it already match, with
  the narrower range to write instead: after `0..=5`, the arm `5..=10` should
  be `6..=10`. A range that surrounds an earlier one is fine
  (`5..10 => a, 0..20 => b`).
- An error for or-pattern alternatives that touch end to end, like
  `0..10 | 10..20`, suggesting the one range `0..20`. `0 | 1 | 2` is fine.

### Changed

- A non-exhaustive `match` on ints names the first range left out
  (``non-exhaustive `match`: `..=-1` not covered``) instead of `_`.

## 0.2.0 (2026-10-14)

Step 2: algebraic data types. See [docs/02-adts.md](docs/02-adts.md).

### Added

- `struct` and `enum` declarations, struct literals (with `Point { x, y }`
  shorthand), field access `p.x`, and variant constructors `Shape::Rect(2, 3)`.
- `match` with nested patterns: `_`, variables, int/bool/`()` literals,
  variants, struct patterns with `..`, and or-patterns `a | b`.
- Patterns in `let`, which must be irrefutable: `let Point { x, y } = p;`.
- Exhaustiveness checking with a counterexample
  (``non-exhaustive `match`: `Shape::Rect(_, _)` not covered``), and errors
  for unreachable arms and or-pattern alternatives.
- The empty type: `enum Never {}`, where `match n {}` needs no arms. Variants
  that hold an empty type need no arm either.
- `==` and `!=` on structs and enums that contain no functions.

### Changed

- An `if`, `match` or block at the start of a statement ends the statement,
  and without a `;` it must have type `()`, as in Rust. Before,
  `if c { 10 } else { 20 }` followed by `- x` on the next line was a
  subtraction.
- The interpreter's stack is larger, so deep recursion in debug builds hits
  the 10,000-call limit instead of overflowing the native stack.

### Not yet

- Recursive types (step 5) and generic types (step 4) are rejected.
- Range patterns are deferred.

## 0.1.0 (2026-10-13)

Step 1: the simply typed lambda calculus. See [docs/01-stlc.md](docs/01-stlc.md).

- Ints, bools, `()`, first-class top-level functions, `let`, `if`/`else` and
  recursion, with a type checker and a tree-walking interpreter.
- `print` and `print_bool` builtins.
- `t lsp`, a language server that reports errors as you type.
