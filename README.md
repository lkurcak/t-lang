# T

Like C, but a different letter.

A small systems language with no garbage collector and no runtime, built one
type-theory concept at a time, mainly to learn what those concepts are and why
they matter.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"Hello, T!") // expect: Hello, T!
}
```

[examples/features.t](examples/features.t) shows enums, pattern matching,
generics and closures; [examples/list.t](examples/list.t) shows ownership
and borrowing.

The [language reference](docs/reference.md) brings together the current
rules for types, inference, ownership, effects, visibility, and refinements.
The [numbered design notes](#roadmap) explain how those rules developed.

## Install

```sh
brew install lkurcak/tap/t-lang
```

or

```sh
cargo install t-lang    # either way, installs the `t` command
```

(`cargo install` also needs a C compiler, for the tree-sitter parser `t`
carries; see [Syntax highlighting](#syntax-highlighting).)

## Usage

```sh
t examples/hello.t          # type-check and run
t check examples/hello.t    # type-check only (--deny-warnings: fail on a warning)
t test examples/hello.t     # type-check, then run each #[test] (see docs/31-tests.md)
t mono examples/hello.t     # print the program after monomorphisation
t build examples/hello.t    # compile, through C, to ./hello (see docs/33-c-backend.md)
t fmt examples/hello.t      # format the file in place (--check: just print a diff)
t lsp                       # language server (see below)
```

A program of several files starts in `main.t`, and `mod geo;` in it is
the module in `geo.t` beside it (see
[docs/06d.1-files.md](docs/06d.1-files.md)).

From a checkout, use `cargo run -- <args>` instead, and `cargo test` to run
every program in `tests/t/` and the code in the docs (see [Tests](#tests)).

`t fmt` formats T in one style, with no options, at 100 columns, keeping
comments and single blank lines. It refuses to write anything that would
parse to a different program. See [docs/fmt.md](docs/fmt.md) for why there
are no options and how it works.

Warnings don't stop a program, and most come with a fix. Each belongs to a
lint, which the first warning of each names. `#[allow(..)]` before a `fn`,
`struct`, `enum`, field or variant turns the lints it names off in all of
it, a function's body and the functions nested in it included:

```rust
#[allow(non_camel_case_types, dead_code)]
#[derive(Discard)]
enum ffi_result { OK, Retry, Fatal }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r = ffi_result::OK;
    match r {
        ffi_result::OK => out.line(&0i64)?, // expect: 0
        _ => out.line(&1i64)?,
    }
    Result::Ok(())
}
```

As in Rust, a name that isn't a lint's is a warning (`unknown_lints`), not
an error. The lints are `unused_variables`, `unused_mut`, `dead_code`,
`unreachable_patterns`, `overlapping_ranges`, `mergeable_ranges`,
`unused_doc_comments`, `non_camel_case_types`, `non_snake_case`,
`infinite_types`, `unreachable_code`, `unused_labels`, `unused_imports`,
`empty_ranges`, `unused_effects` and `unknown_lints`.

Pipeline: `lexer` → `parser` → `check` (type checker, then `borrowck`, the
borrow checker, which lowers each body to MIR, checks it, and keeps it) →
`lift` (nested functions and types moved to the top level) → `mono`
(monomorphisation: a copy of each generic function's MIR and of each type
per type it's used at) → `interp` (an interpreter of that MIR, which checks
memory as it goes), or `cgen` (C, for `t build`, from the same MIR). So
what runs is what the borrow checker checked (docs/32-mir.md). What's
dropped where is worked out before running, with drop flags where it
depends on the path, and dropping and `==` are MIR functions too
(docs/32.1-drops.md), so the C backend translates the MIR a statement at
a time, and decides nothing of its own (docs/33-c-backend.md).

`t check` stops after semantic checking, including effects, refinements and
borrows. Running a program and `t mono` then lower it for execution. The
[analysis API and LSP snapshots](docs/analysis.md) keep those phases separate.

## Editor support

`t lsp` is a Language Server Protocol server over stdio. It reports errors and
warnings as you type, and offers their fixes as quick fixes (code actions).
Semantic requests share the program's analysis until an open document or a
disk dependency changes; closing an unsaved file restores its disk contents.
Hovering shows types, and the doc comment of a function, struct, enum,
field or variant: a `///` comment just before it, written in Markdown.
Go to definition works for variables, functions, types, fields,
variants and type parameters, and go to type definition jumps from a
variable, field or function to the structs and enums in its type (each of
them, for one like `Option<Point>`). Find references and document
highlights show everywhere something is named, and rename renames it
there, following what each name refers to rather than its spelling: of
two nested `fn helper`s, or a local `struct P` and a top-level one, only
the one you're on. Renaming a variable in a shorthand `P { x }` gives
`P { x: new }`, and renaming the field gives `P { new: x }`. A rename that
would change what any name refers to, such as one that shadows or clashes
with something, is refused with what it would clash with, and comments
are left alone. The outline (document symbols) lists the functions,
structs and enums, with the fields and variants of each type and the
nested functions and local types of each function under them; it comes
from the parse alone, keeping complete declarations around malformed items
while reporting their syntax errors. Formatting and semantic analysis still
require valid syntax. Inlay hints show inferred types and type arguments.
It formats files as `t fmt` does (see below).

Definition and type-definition navigation also open the embedded `core`,
`alloc` and `std` sources, including library methods, types and imports.
Module imports open the module's file. At startup, `t lsp` writes read-only
copies of its embedded sources to a private directory under the system's
temporary directory and returns ordinary `file:` URIs, so no editor extension
or compiler checkout is required. These are the exact sources built into the
running binary, with their original bytes and source positions. Each server
session gets its own directory, including for different builds of the same
compiler version. The copies remain after shutdown until the temporary
directory is cleared, so existing links and buffers can still be read.

Opened library files support further navigation, hovers and outlines using
the embedded sources. They are for browsing: rename, formatting and code
actions do not edit them, and editor changes to them do not affect analysis.
Find references remains scoped to the program's own files.

Names are written as in Rust: structs, enums, variants and type
parameters in `UpperCamelCase`, and functions, parameters, variables and
fields in `snake_case`, with an acronym as one word (`HttpServer`), and
leading underscores and digits allowed (`_unused`, `Vec2`). A name written
otherwise gets a warning where it's declared (`non_camel_case_types` or
`non_snake_case`), whose fix is the rename above, unless the new name
would be a keyword or clash with something.

Acronyms are one word even when the whole name is one: `Io`, `Id`, `Ui`,
not `IO`, `ID`, `UI`. Rust's style guide says the same (`Uuid`,
`TcpStream`), but rustc doesn't enforce it; T does, so that each name has
one right spelling and its case says what it is. `Io` is a type, `io` a
function or variable, and all capitals are kept for globals, should T get
them.

**Neovim (0.11+):** install
[t-lang.nvim](https://github.com/lkurcak/t-lang.nvim), e.g.
`vim.pack.add({ "https://github.com/lkurcak/t-lang.nvim" })`.

**Helix:** merge [editors/helix/languages.toml](editors/helix/languages.toml)
into `~/.config/helix/languages.toml`.

**Anything else with an LSP client:** run `t lsp` for files ending in `.t`.

### Formatting

`t lsp` answers "format document" requests with `t fmt`'s formatting. It
ignores the editor's formatting options, such as tab size: there's one
style, with no options.

**Neovim:** `:lua vim.lsp.buf.format()` formats the current file. To format
on every save:

```lua
vim.api.nvim_create_autocmd("BufWritePre", {
  pattern = "*.t",
  callback = function() vim.lsp.buf.format() end,
})
```

**Helix:** `:format` (`:fmt`) formats the current file.
[editors/helix/languages.toml](editors/helix/languages.toml) sets
`auto-format = true`, so Helix also formats on every save.

### Syntax highlighting

[editors/tree-sitter-t](editors/tree-sitter-t) is a
[tree-sitter](https://tree-sitter.github.io/) grammar for T, with highlight
queries in `queries/highlights.scm` that work in both editors below. `t`
carries the parser, built from it, and its queries: `t tree-sitter` installs
them, so what's highlighted matches what that `t` accepts, and needs no
network or C compiler. Run it again after you upgrade `t`. The generated
parser (`src/parser.c`) is checked in. After changing `grammar.js`, regenerate it with
`tree-sitter generate` in that directory; `cargo test` checks that it parses
every program in `tests/t/` and `examples/`.

**Helix:** with [editors/helix/languages.toml](editors/helix/languages.toml)
merged into your config, install the parser and queries into Helix's
`runtime` directory (`%AppData%\helix\runtime` on Windows):

```sh
t tree-sitter --helix ~/.config/helix/runtime
```

**Neovim (0.11+):** [t-lang.nvim](https://github.com/lkurcak/t-lang.nvim)
installs the parser the first time you open a T file, and again after you
upgrade `t`. Without it, `t` installs the parser it was built with, and its
highlight queries, into your `site` directory:

```sh
t tree-sitter ~/.local/share/nvim/site
```

and turn it on for T files in your config:

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "t",
  callback = function() vim.treesitter.start() end,
})
```

## Tests

`cargo test --lib parser::generated_tests` runs bounded generated tests for
strict parsing, editor recovery, and formatting; they also run with the
normal test suite. Two fixed seeds generate 64 programs at expression depth
at most three. They mutate 16 delimiter positions per program, truncate
four programs at every UTF-8 character boundary, and build 512 short
fragment sequences. Literal and nested-module matrices cover Unicode, escapes,
comments, and recovery around incomplete items. Inputs are capped at 8 KiB.
Failures print the seed/case and exact source to reproduce; no external
fuzz runner or new dependencies are needed.

`cargo test --lib check::refine::logic::generated_tests` checks solver
soundness against exhaustive evaluation over 50 assignments for each of
512 generated questions, with two fixed seeds and both small and large
coefficients. Reversed assumptions and atoms at `i128` limits exercise
conservative failure too; an unproved true condition is allowed.
`cargo test --test generated_semantics` checks integer boundary guards,
all 24 declaration orders of programs combining mutation, borrowing,
closures, refinements, and early returns, and paired valid/invalid closure
lifetime cases. Accepted programs run and match independently calculated
output; invalid programs must fail during type checking. These tests also
run in the normal suite, cap source inputs at 4 KiB, and report the exact
question or program on failure.

Each `tests/t/*.t` file is a program with its expectations in comments:

```rust,fragment
out.line(&(1 + 2))?; // expect: 3
// stderr: error: broken pipe
// stdin: a line of input
// error: expected `i64`, found `bool`
// warning: unreachable pattern
```

`// expect:` lines are what it writes to its output, `// stderr:` lines
what it writes where errors go, and `// stdin:` lines its input. With
`// test: name ... ok` or `// test: name ... FAILED: text` lines, one per
test, its `#[test]`s are run too, as `t test` runs them.

Every warning must be expected, and applying the warnings' fixes must not
change what the program does. A program of several files is a directory,
`tests/t/<name>/main.t` with its modules' files, and its comments are read
from all of them. A `tests/t/<name>.mono` file holds what
`t mono` prints for that test; `BLESS=1 cargo test` rewrites them.

Every one of those programs that runs is also compiled by the C backend
and run, and must do exactly what the interpreter does (`tests/compiled.rs`,
which needs a C compiler); CI does it again with AddressSanitizer and
UndefinedBehaviorSanitizer (`T_LANG_CFLAGS`).

The T code in this README and in `docs/` is tested too (`tests/docs.rs`).
Every ```` ```rust ```` block must parse and be formatted as `t fmt` would,
and one with a `fn main` must run, with the same comments as above. Words
after `rust` say what else a block is; GitHub reads only the first word, so
they don't show:

| Fence | The block is | Checked |
|---|---|---|
| ```` ```rust ```` | items, or a whole program | it runs if it has a `fn main` |
| ```` ```rust,fragment ```` | statements, as in a function body | as the body of a `fn main`, not run |
| ```` ```rust,error ```` | a program shown for its error | it fails with its `// error:`; an empty `main` is added if it has none |
| ```` ```rust,unformatted ```` | laid out badly on purpose | not for formatting |

They combine, as in ```` ```rust,fragment,unformatted ````. Blocks in other
languages, such as ```` ```text ````, aren't checked.

Code in doc comments (`///`) isn't run yet. Doctests are the natural next
step: the code blocks in a doc comment are Markdown like these, and
`tests/docs.rs` already knows how to check one.

## Design constraints

T should be good enough to write an operating system in, so it has to
compile to bare metal: no garbage collector, and no runtime. Every feature
has to fit that:

- **Every value has a size known at compile time.** Generics are
  monomorphized, so there's no boxing and no type information at run time.
- **Allocation is an effect.** `Box::new(x)` and the allocating operations
  of `List`, `String` and `Arena` say `with alloc`. Ownership and
  destructors free their memory: no garbage collector, no reference
  counting. `Box` is the alloc library's,
  `alloc::boxed::Box`, as in Rust's split between `core`, `alloc` and
  `std`. See
  [docs/05-ownership.md](docs/05-ownership.md). An arena,
  `alloc::arena::Arena`, holds many values that are freed together and
  can point to each other (see [docs/23-arenas.md](docs/23-arenas.md)).
- **Closures are values of their own types**, structs of what they borrow
  or own, called through the `Fn` traits, like Rust's: no closure needs the
  heap. The borrow checker sees what each borrows. See
  [docs/06c-closures.md](docs/06c-closures.md). A function returns one as
  `impl Fn(i64) -> i64`, a type its callers know only by what it can do
  (see [docs/06c.1-opaque-types.md](docs/06c.1-opaque-types.md)).
- **Errors are values, not exceptions.** There's no unwinding. A panic
  aborts, and a function's type says if it may: `fn mean(a: i64, b: i64) ->
  i64 with panic`. Allocating is recorded the same way, `with alloc`.
  Non-exported functions, including `main`, infer their effects when no
  `with` is written; `with {}` explicitly promises none. Exported
  functions and trait methods have only the effects their signatures say.
  See [docs/07-effects.md](docs/07-effects.md) and
  [docs/24-visibility.md](docs/24-visibility.md). Arithmetic
  the checker proves can't overflow, from refinement types like `{ d: i64
  | d > 0 }`, can't panic (see
  [docs/08-refinement-types.md](docs/08-refinement-types.md)). A struct's
  fields can have them too, which every value of it keeps (see
  [docs/08.1-field-refinements.md](docs/08.1-field-refinements.md)). And
  they can name *measures*, methods like `len()`, so `s[i]` is proved in
  range and can't panic either (see
  [docs/21-measures.md](docs/21-measures.md)), for arrays too, `[T; N]`,
  whose length is in their type (see [docs/22-arrays.md](docs/22-arrays.md)).
- **Values must be used up.** One whose type isn't `Discard` can't be
  dropped or overwritten: a resource like a file descriptor can't be
  forgotten. See [docs/06a.1-linear-types.md](docs/06a.1-linear-types.md).
  A type that is says how it's thrown away, field by field with
  `#[derive(Discard)]`, or with a `discard` of its own, as a list frees
  its buffer, which can't fail. See
  [docs/20-destructors.md](docs/20-destructors.md).
- **The core library is written in T.** `Option`, `Result`, `Iterator` and
  the rest are checked like any program; all the checker takes on trust is
  its intrinsics, `swap`, `panic`, `checked_mul`, `least`, `greatest`, a
  slice's few and a string's length, and the arithmetic operators. See
  [docs/06e-core.md](docs/06e-core.md).
- **Integers say how big they are.** `i8` to `i64`, `u8` to `u64`, `isize`
  and `usize`, and no `int`: an operator gives the number it does in ℤ, or
  panics, and never wraps; `x as u8` is the same number or a panic. What the
  checker proves of `usize` holds on 32- and 64-bit machines alike. See
  [docs/11-integers.md](docs/11-integers.md).
- **Unsafe code is marked, and small.** What the checker can't check, like
  which slots of a raw buffer hold values, is `unsafe`, an effect only an
  `unsafe` block takes away, so the trust is exactly those blocks. The
  `alloc` library builds on `RawBuf<T>`, the one way to get memory for
  many values. See [docs/09-unsafe.md](docs/09-unsafe.md). Its `List<T>`
  is a safe interface over it, which nothing outside can break (see
  [docs/09.1-list.md](docs/09.1-list.md)), and its values are seen as a
  slice, `&[T]` (see [docs/10-slices.md](docs/10-slices.md)).
- **No ambient authority.** A program reads and writes only through the
  streams `main` is given, `fn main(os: Os)`, which no code can make or
  copy: a function that isn't handed one can't do I/O. Writing gives an
  error as a value, `out.line(&x)?`. See
  [docs/19-std-io.md](docs/19-std-io.md).

Within that, the aim is to push the type system as far as it goes, and keep
it sound: well-typed programs don't go wrong, including the ones that
combine features in ways nobody planned for.

## Roadmap

| Step | Feature | Concept | Notes |
|---|---|---|---|
| 1 ✅ | ints, bools, `let`, functions, `if` | simply typed lambda calculus, soundness | [docs/01-stlc.md](docs/01-stlc.md) |
| 2 ✅ | structs, enums, `match` | algebraic data types, exhaustiveness | [docs/02-adts.md](docs/02-adts.md) |
| 3 ✅ | inference, closures, nested `fn`s and types | unification / bidirectional typing | [docs/03-inference.md](docs/03-inference.md) |
| 4 ✅ | generics | rank-1 (prenex) parametric polymorphism, monomorphization | [docs/04-generics.md](docs/04-generics.md) |
| 5 ✅ | ownership, borrowing, pointers | affine types, regions/lifetimes; types that hold themselves through a pointer | [docs/05-ownership.md](docs/05-ownership.md) |
| 5.1 ✅ | `return`, `break`, `continue`, `loop`, labels | the empty type `!`, divergence | [docs/05.1-control-flow.md](docs/05.1-control-flow.md) |
| 6a ✅ | traits, impls, methods, `Eq` and `Copy` | ad-hoc polymorphism, type classes, coherence, termination of resolution; a call's borrows taken after its arguments | [docs/06-traits.md](docs/06-traits.md) |
| 6a.1 ✅ | values must be used up, unless `Discard`; `replace` | linear types, substructural rules (weakening, contraction) | [docs/06a.1-linear-types.md](docs/06a.1-linear-types.md) |
| 6b ✅ | associated types, projections, equality bounds | type-level functions (open type families), normalisation with lifetimes | [docs/06b-associated-types.md](docs/06b-associated-types.md) |
| 6c ✅ | closures as `Fn` traits | closure conversion, a type per closure, capturing by reference | [docs/06c-closures.md](docs/06c-closures.md) |
| 6c.1 ✅ | returning closures, `-> impl Fn(i64) -> i64` | existential (opaque) types, which lifetimes they capture | [docs/06c.1-opaque-types.md](docs/06c.1-opaque-types.md) |
| 6d ✅ | modules: `mod`, `use`, `pub` | namespaces, abstract types (existential types, representation independence), coherence and orphan rules | [docs/06d-modules.md](docs/06d-modules.md) |
| 6d.1 ✅ | modules in files of their own: `mod m;` | a file is the module's braces; one file per module path | [docs/06d.1-files.md](docs/06d.1-files.md) |
| 6e ✅ | a small `core` library: `Option`, `Result`, `Ord`, `Iterator`, `for`, `a..b`, `where` clauses | a library written in T with one trusted intrinsic, a prelude that can't change a program, orphan rules between packages, `for` as a desugaring | [docs/06e-core.md](docs/06e-core.md) |
| 6e.1 ✅ | closures that take a reference of any lifetime; `Iterator::filter` | higher-ranked closure types: a closure's body checked for every lifetime, with placeholders; its result's lifetimes inferred from its body, then checked; signatures equal up to renaming | [docs/06e.1-higher-ranked-closures.md](docs/06e.1-higher-ranked-closures.md) |
| 7 ✅ | `with panic`, `with alloc`: what a call may do, in its type; `panic()`, `unwrap`, `checked_add` | effect systems, effect polymorphism (effects as a kind, least solutions), associated effects; divergence left for a termination checker | [docs/07-effects.md](docs/07-effects.md) |
| 7.1 ✅ | impl parameters a bound decides, `where` on impls; `Map<I, F>` and `Filter<I, P>` | functional dependencies (a parameter as a function of the type), termination of resolution with bounds on projections (strongly connected components of the trait graph) | [docs/07.1-impl-parameters.md](docs/07.1-impl-parameters.md) |
| 8 ✅ | refinement types, `{ d: i64 \| d > 0 }`, on parameters, results and `let`s; arithmetic proved total | refinement types, verification conditions from facts along each path (static single assignment), linear integer arithmetic decided by Fourier–Motzkin with integer rounding: sound, incomplete | [docs/08-refinement-types.md](docs/08-refinement-types.md) |
| 8.1 ✅ | refinements on struct fields, `hi: { h: i64 \| h >= lo }`; facts through field reads | invariants of a type, kept by every value; facts about places (fields, `*`), forgotten where they may change | [docs/08.1-field-refinements.md](docs/08.1-field-refinements.md) |
| 9 ✅ | `unsafe` blocks, `with unsafe`; an `alloc` library with `RawBuf<T>`, a buffer of slots | unsafe code as an effect, discharged only by a block; a small trusted base, and its obligations checked as the program runs (as Miri does) | [docs/09-unsafe.md](docs/09-unsafe.md) |
| 9.1 ✅ | `alloc::list::List<T>`, a dense, resizable array, and iterating it | a safe interface over unsafe code, kept by a type's invariant (representation invariants, as RustBelt's), which privacy, borrows and linearity protect | [docs/09.1-list.md](docs/09.1-list.md) |
| 10 ✅ | slices, `&[T]` and `&mut [T]`; `List::as_slice`, `iter_mut` | a kind of types that are only ever behind a reference, so no `?Sized`; splitting a unique borrow into disjoint ones (`split_at_mut`), so `iter_mut` is safe code | [docs/10-slices.md](docs/10-slices.md) |
| 11 ✅ | integer types `i8`..`u64`, `isize`, `usize`, no `int`; literals inferred, or `255u8`; `x as u8`; `T: Int` | a family of base types, each an interval of ℤ; literals typed by inference with no default; proofs about a size the machine decides, by a symbolic bound; generic code over a closed (sealed) trait | [docs/11-integers.md](docs/11-integers.md) |
| 12 ✅ | bitwise `&`, `\|`, `^`, `!`, shifts `<<` `>>`; `0xFF`, `0b1010`; `wrapping_add` and friends; `T: Signed`, `T: Unsigned` | bitwise operations on two's complement as operations in ℤ (2-adic integers), so the same rule as arithmetic; precedence as a partial order; non-linear operators known to a linear logic by lemmas | [docs/12-bits.md](docs/12-bits.md) |
| 13 ✅ | strings: `str`, `char`, `"text"`, `'a'`, `b"bytes"`, escapes; `chars`, `from_utf8`; `panic("why")` | a type's invariant (UTF-8) kept by what can make and change its values, as a safety invariant unsafe code relies on; a type with a gap in its range, known to a linear logic by a disjunction | [docs/13-strings.md](docs/13-strings.md) |
| 13.1 ✅ | `alloc::string::String`, an owned, growing string: `push`, `push_str`, `pop`, `truncate`, `from_utf8` | an `unsafe` block whose proof rests on safe code: the invariant's reach is the module that can touch the state, which privacy bounds | [docs/13.1-string.md](docs/13.1-string.md) |
| 14 ✅ | each package checked once, against those it depends on: core, then alloc, then the program | separate compilation: whole-program rules (no cycles, no overlap) split into one per package, so a library that checked stays checked, and an error is the newest package's | [docs/14-packages.md](docs/14-packages.md) |
| 15 ✅ | `e?`: what's in an `Ok` or `Some`, or else return early; `core::ops::Try`, `ControlFlow` | early return as a desugaring through a trait; residuals (`Result<!, E>`, `Option<!>`), uninhabited types keeping kinds of failure apart | [docs/15-try.md](docs/15-try.md) |
| 16 ✅ | running out of memory is part of `alloc`, not `panic`: `Box::new`, `List::push`, `String::push` say `with alloc`; `RawBuf::grow(extra)`, `alloc::raw::out_of_memory` | which effect an abort is: `panic` for bugs, `alloc` for resources, and the library's signatures proved without `panic` | [docs/16-out-of-memory.md](docs/16-out-of-memory.md) |
| 17 ✅ | retrospective 1: effect parameters declared `effect e`; `Discard` and `Copy` only derived; `//!` module docs | kinds given by declaration, not spelling; marker traits whose proofs are closed under their own trait, so no cycle can run through them | [docs/17-retrospective-1.md](docs/17-retrospective-1.md) |
| 17.1 ✅ | `Discard`, `Copy`, `Eq`, `Int`, `Fn`… are core's `#[intrinsic]` traits, in the prelude; `Box` is `alloc::boxed::Box`; `<` is `Ord`'s, for any ordered type | the checker knows items by symbol, never by name; operators as trait methods, with the machine's `<` on integers the only one the refinement logic reads | [docs/17.1-library-items.md](docs/17.1-library-items.md) |
| 18 ✅ | `core::fmt`: `Write`, a sink of text, and `Display`, for integers, `bool`, `char`, strings and a program's own types; `w.put(&x)?`, `String::of(&x)` | effects bounded by an argument's: formatting does only what its sink does, so into a pure sink it's total; numbers in decimal proved not to panic, at every integer type; each sink's errors a type of its own | [docs/18-fmt.md](docs/18-fmt.md) |
| 19 ✅ | `std`: `fn main(os: Os)`, `out.line(&x)?`, `stdin.read_line(&mut line)?`, `main` giving back `Result<(), E>`; the `print` built-ins are gone | capabilities: what a function can do to the world is what it's handed, values no code can forge or copy (uniqueness, as Clean's world), not an effect; errors as values meeting linear types | [docs/19-std-io.md](docs/19-std-io.md) |
| 20 ✅ | destructors: `impl Discard for T { fn discard(self) { .. } }`, called wherever a value is dropped; `List` and `String` are `Discard`, so `?` works while one is alive; `x.discard()` | weakening as a map `A ⊸ 1`, inserted only where it's total; a drop as a use of what's borrowed (drop check); only a type's module may skip its destructor | [docs/20-destructors.md](docs/20-destructors.md) |
| 21 ✅ | measures: `#[measure] fn len(&self) -> usize`, named in refinements, `i: { i: usize \| i < s.len() }`; refinements of any type; `s[i]`, `l[i] = v`, proved in range | measures as functions of a value, uninterpreted outside their module (abstract functions); equality with uninterpreted functions by object identity; a `&mut`'s place forgotten where it's lent, as the borrow checker says no one reads it meanwhile | [docs/21-measures.md](docs/21-measures.md) |
| 22 ✅ | arrays: `[T; N]`, inline; `[a, b, c]`, `[v; n]`, `let [a, b] = arr;`, `a[i]` proved from `N`; `const N: usize` parameters on functions, structs and impls, a `usize` in bodies and refinements; `core::array::from_fn` | type-level naturals as a kind of type parameter, equal only syntactically, so unification stays first-order; a value's length from its type, a constant of the logic or one variable per const parameter | [docs/22-arrays.md](docs/22-arrays.md) |
| 23 ✅ | arenas: `alloc::arena::Arena<T>`, `a.alloc(v)` handing out `&mut T`s that live as long as the arena, so its values can point to each other; dropped all at once; a design note on allocators as values | region-based memory, the regions being lifetimes; fresh `&mut`s from a `&`, so invariance as a `Cell`'s; the drop check looking into an intrinsic type's arguments, Rust's `#[may_dangle]` decided by what a `discard` can reach | [docs/23-arenas.md](docs/23-arenas.md) |
| 24 ✅ | visibility and effect inference: `pub` is seen in the package, `export` by other packages (the interface, reachable from the root); a function that isn't exported and says no `with` has its body's effects, `with {}` says none | two boundaries, module and package, so three levels; effects as the least fixed point over the call graph's strongly connected components, checked callees first; what waits for a recursive group checked at the least solution | [docs/24-visibility.md](docs/24-visibility.md) |
| 25 ✅ | type aliases: `type Nodes<'a> = Arena<Expr<'a>>;`, with lifetime, type and const parameters, at the top of a module or in a block; `pub` and `export` as for any item | transparent synonyms, expanded where types are resolved, so every check sees the program written out; hygienic, names resolved at the alias; acyclic, closed and bound-free, so an alias means one thing, a function of its arguments | [docs/25-type-aliases.md](docs/25-type-aliases.md) |
| 26 ✅ | packages: a `t.toml` beside `main.t` or `lib.t` lists `[dependencies]` by path; a library starts in `lib.t` and checks on its own, with no `main`; the name a dependent gives a package starts its paths | separate compilation with packages someone else wrote: each read once per program, by directory, so a type is one type everywhere; checked in an order where each comes after what it depends on, and, for now, a chain, the shape in which a package checks the same alone as in any program | [docs/26-packages.md](docs/26-packages.md) |
| 27 ✅ | git dependencies: `name = { git = "url", rev = "<full hash>" }` in a `t.toml`, fetched once by `git` into a cache (`$T_LANG_CACHE`, or the system's, in `t-lang`) and shown as `<name@rev>` | content addressing as identity: a full commit hash pins a package's files, so the manifest is its own lock; fetching runs no code (no hooks, no `ext::`, no links out of a checkout), one commit is one package, and two revs of one repository are an error, for now | [docs/27-git-dependencies.md](docs/27-git-dependencies.md) |
| 28 ✅ | `if let`, `else if let`, `while let` and `let .. else`, whose `else` must leave the block; facts about `%` by any divisor: `h % n < n` for `n > 0` | desugaring as the whole semantics: each is the `match` it means, its left-out arm the variants the pattern misses, taken apart so linear values are used up; "must leave" as the type `!`; truncated remainder's bounds, linear and true at every width | [docs/28-if-let.md](docs/28-if-let.md) |
| 29 ✅ | closed and sealed traits: an impl's bound on what isn't part of its type only of `Discard`, `Copy`, `Fn*` or the integer traits; `sealed trait`, implemented only by its package; one `impl<T: Int>` beside others; `core::num::wrap`; packages that don't know each other in one program | locality: every rule decided by a package and its dependencies, so adding a dependency never breaks a program; Paterson conditions extended by proof-closed traits; polymorphic recursion over dictionaries, edges from where a proof picks an impl; negative reasoning from a closed world, one unification deep | [docs/29-closed-and-sealed-traits.md](docs/29-closed-and-sealed-traits.md) |
| 30 ✅ | postconditions: `ensures`, `old(..)`, on what a call leaves behind a `&mut` that can't outlive it; a field a refinement is about lent to such a call, its refinement proved again after; `List`'s `len <= cap()` as a refinement, `push` without its overflow check; `RawBuf`'s bounds as refinements | call-scoped borrows read off lifetimes; invariants broken only while no one can look (no unwinding); prophecies (RustHorn) for intrinsics; why writing through a `&` needs invariance (`Cell`) | [docs/30-postconditions.md](docs/30-postconditions.md) |
| 31 ✅ | tests: `#[test]` functions, each run by `t test` as a program of its own, with no `Os`; `#[test(panics)]`; `assert`, `assert_eq`, `assert_ne` in the prelude, through `panic_with` and a sink that can't fail or allocate; a panic in the built-in libraries shown at the caller; `--deny-warnings` | a test as a program: isolation from no I/O and no globals, not from catching panics; one meaning, no `cfg`; caller location without `#[track_caller]` | [docs/31-tests.md](docs/31-tests.md) |
| 32 ✅ | one MIR: the borrow checker's MIR gets what running needs (constants, operators' nodes, aggregates, match patterns, refinements, `unsafe` spans) and is kept; mono copies MIR; the interpreter runs it; the AST interpreter and `lift`'s body rewriting gone; `t mono` prints MIR | borrow checking the code that runs, not a second lowering that must agree; monomorphisation changes no control flow; generic meaning wins by construction | [docs/32-mir.md](docs/32-mir.md) |
| 32.1 ✅ | drop elaboration: move paths and an init dataflow on the generic MIR after borrow checking; `drop`s where values are definitely there, drop flags where it depends on the path, open drops of what's left; drop and `==` glue as MIR functions from mono; `from_fn` and closure calls no longer drop behind the scenes | flags track exactly what's there, so elaborated drops are the old dynamic ones, in the same order, checked by the interpreter as it runs; decisions only from moves, so the same in every copy | [docs/32.1-drops.md](docs/32.1-drops.md) |
| 33 ✅ | C backend: `t build` compiles, through one C11 file and `$CC`, for 64-bit Linux and macOS; the MIR translated a statement at a time, with a small runtime; panics shown as the interpreter shows them; every program of the tests and docs run both ways and compared, and again under sanitizers | one semantics, translated not reimplemented; C without undefined behaviour by construction (checked arithmetic in wider types, defined shifts, no punning, zeroed locals); proofs never trusted with more than a wrong number; differential testing | [docs/33-c-backend.md](docs/33-c-backend.md) |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
