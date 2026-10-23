# T

Like C, but a different letter.

A small systems language with no garbage collector and no runtime, built one
type-theory concept at a time, mainly to learn what those concepts are and why
they matter.

```rust
enum Shape { Circle(i64), Rect(i64, i64), Empty }

/// The area of `s`, with π rounded down to 3.
fn area(s: Shape) -> i64 with panic {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}

fn fact(n: i64) -> i64 with panic {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn twice<e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(f(x)) }

fn main() with panic {
    let n = 5;
    print(fact(10));                                    // expect: 3628800
    print(twice(fact, 3));                              // expect: 720
    print(twice(|x| x + n, 0));                         // expect: 10
    print(twice(square, 3));                            // expect: 81
    print(area(Shape::Rect(3, 4)));                     // expect: 12
    print(area(Shape::Circle(2)) + area(Shape::Empty)); // expect: 12

    // A nested `fn`: only `main` can see it.
    fn square(x: i64) -> i64 with panic { x * x }
}
```

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
t check examples/hello.t    # type-check only
t mono examples/hello.t     # print the program after monomorphisation
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

fn main() {
    let r = ffi_result::OK;
    match r {
        ffi_result::OK => print(0i64), // expect: 0
        _ => print(1i64),
    }
}
```

As in Rust, a name that isn't a lint's is a warning (`unknown_lints`), not
an error. The lints are `unused_variables`, `unused_mut`, `dead_code`,
`unreachable_patterns`, `overlapping_ranges`, `mergeable_ranges`,
`unused_doc_comments`, `non_camel_case_types`, `non_snake_case`,
`infinite_types`, `unreachable_code`, `unused_labels`, `unused_imports`,
`empty_ranges`, `unused_effects` and `unknown_lints`.

Pipeline: `lexer` → `parser` → `check` (type checker, then `borrowck`, the
borrow checker) → `lift` (nested functions and types moved to the top
level) → `mono` (monomorphisation: a copy of each generic function and type
per type it's used at) → `interp` (tree-walking interpreter, which checks
memory as it goes). A real backend (emitting C) comes once the type system
has enough in it to be worth compiling.

## Editor support

`t lsp` is a Language Server Protocol server over stdio. It reports errors and
warnings as you type, and offers their fixes as quick fixes (code actions).
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
from the parse alone, so it's there while the file has a type error. Inlay
hints show inferred types and type arguments. It formats files as `t fmt`
does (see below).

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

Each `tests/t/*.t` file is a program with its expectations in comments:

```rust,fragment
print(1 + 2); // expect: 3
// error: expected `i64`, found `bool`
// warning: unreachable pattern
```

Every warning must be expected, and applying the warnings' fixes must not
change what the program does. A program of several files is a directory,
`tests/t/<name>/main.t` with its modules' files, and its comments are read
from all of them. A `tests/t/<name>.mono` file holds what
`t mono` prints for that test; `BLESS=1 cargo test` rewrites them.

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
- **Nothing allocates behind your back.** `Box::new(x)` is the only thing
  that touches the heap, and the box is freed when its owner goes away,
  which the borrow checker works out at compile time: no garbage collector,
  no reference counting. `Box` will move to a library, as in Rust's split
  between `core`, `alloc` and `std`. See
  [docs/05-ownership.md](docs/05-ownership.md).
- **Closures are values of their own types**, structs of what they borrow
  or own, called through the `Fn` traits, like Rust's: no closure needs the
  heap. The borrow checker sees what each borrows. See
  [docs/06c-closures.md](docs/06c-closures.md). A function returns one as
  `impl Fn(i64) -> i64`, a type its callers know only by what it can do
  (see [docs/06c.1-opaque-types.md](docs/06c.1-opaque-types.md)).
- **Errors are values, not exceptions.** There's no unwinding. A panic
  aborts, and a function's type says if it may: `fn mean(a: i64, b: i64) ->
  i64 with panic`. One that doesn't say so can't, nor can a `main` that
  doesn't, nor anything it calls; allocating is in types the same way,
  `with alloc`. See [docs/07-effects.md](docs/07-effects.md). Arithmetic
  the checker proves can't overflow, from refinement types like `{ d: i64
  | d > 0 }`, can't panic (see
  [docs/08-refinement-types.md](docs/08-refinement-types.md)). A struct's
  fields can have them too, which every value of it keeps (see
  [docs/08.1-field-refinements.md](docs/08.1-field-refinements.md)).
- **Values must be used up.** One whose type isn't `#[derive(Discard)]`
  can't be dropped or overwritten: a resource like a file descriptor can't
  be forgotten. See [docs/06a.1-linear-types.md](docs/06a.1-linear-types.md).
- **The core library is written in T.** `Option`, `Result`, `Iterator` and
  the rest are checked like any program; all the checker takes on trust is
  its intrinsics, `swap`, `panic`, `checked_mul`, `least`, `greatest` and a
  slice's few, and the arithmetic operators. See
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
| — | `std` | | |
| — | C backend | | when it's worth it |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
