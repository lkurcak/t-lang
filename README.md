# T

Like C, but a different letter.

A small systems language with no garbage collector and no runtime, built one
type-theory concept at a time, mainly to learn what those concepts are and why
they matter.

```rust
enum Shape { Circle(int), Rect(int, int), Empty }

/// The area of `s`, with π rounded down to 3.
fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}

fn fact(n: int) -> int {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn twice(f: fn(int) -> int, x: int) -> int { f(f(x)) }

fn main() {
    let n = 5;
    print(fact(10));                                    // expect: 3628800
    print(twice(fact, 3));                              // expect: 720
    print(twice(|x| x + n, 0));                         // expect: 10
    print(twice(square, 3));                            // expect: 81
    print(area(Shape::Rect(3, 4)));                     // expect: 12
    print(area(Shape::Circle(2)) + area(Shape::Empty)); // expect: 12

    // A nested `fn`: only `main` can see it.
    fn square(x: int) -> int { x * x }
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

## Usage

```sh
t examples/hello.t          # type-check and run
t check examples/hello.t    # type-check only
t mono examples/hello.t     # print the program after monomorphisation
t fmt examples/hello.t      # format the file in place (--check: just print a diff)
t lsp                       # language server (see below)
```

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
        ffi_result::OK => print(0), // expect: 0
        _ => print(1),
    }
}
```

As in Rust, a name that isn't a lint's is a warning (`unknown_lints`), not
an error. The lints are `unused_variables`, `unused_mut`, `dead_code`,
`unreachable_patterns`, `overlapping_ranges`, `mergeable_ranges`,
`unused_doc_comments`, `non_camel_case_types`, `non_snake_case`,
`infinite_types`, `unreachable_code`, `unused_labels` and `unknown_lints`.

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
queries in `queries/highlights.scm` that work in both editors below. The
generated parser (`src/parser.c`) is checked in, so all you need is a C
compiler. After changing `grammar.js`, regenerate it with
`tree-sitter generate` in that directory; `cargo test` checks that it parses
every program in `tests/t/` and `examples/`.

**Helix:** [editors/helix/languages.toml](editors/helix/languages.toml)
already points Helix at the grammar. Then fetch and build it, and copy the
queries where Helix looks for them:

```sh
hx --grammar fetch && hx --grammar build
mkdir -p ~/.config/helix/runtime/queries/t
cp editors/tree-sitter-t/queries/highlights.scm ~/.config/helix/runtime/queries/t/
```

**Neovim (0.11+):** with [t-lang.nvim](https://github.com/lkurcak/t-lang.nvim),
run `:TInstallParser`. Without it, build the parser and copy the queries into
your `site` directory, from a checkout:

```sh
site=~/.local/share/nvim/site
mkdir -p $site/parser $site/queries/t
cc -shared -fPIC -O2 -I editors/tree-sitter-t/src \
    editors/tree-sitter-t/src/parser.c -o $site/parser/t.so
cp editors/tree-sitter-t/queries/highlights.scm $site/queries/t/
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
// error: expected `int`, found `bool`
// warning: unreachable pattern
```

Every warning must be expected, and applying the warnings' fixes must not
change what the program does. A `tests/t/<name>.mono` file holds what
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
- **Closures that capture are downward-only:** they can be passed to
  functions but not returned or stored, since their variables live on the
  stack. A `fn` nested in a block captures nothing, so it can go anywhere.
  See [docs/03-inference.md](docs/03-inference.md).
- **Errors are values, not exceptions.** There's no unwinding.
- **Values must be used up.** One whose type isn't `#[derive(Discard)]`
  can't be dropped or overwritten: a resource like a file descriptor can't
  be forgotten. See [docs/06a.1-linear-types.md](docs/06a.1-linear-types.md).

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
| 6c | closures as `Fn` traits | a type per closure, capturing by reference | |
| 6d | modules: `mod`, `use`, visibility | abstract types, orphan rules (coherence across modules) | derived bounds written out at module boundaries |
| 6e | a small `core`: `Option`, `Result`, `Ord`, `Iterator`, `for` | equality bounds with projections (`IntoIterator`), iterating over linear values | what the tests keep defining themselves; no allocation |
| 7 | effects | effect systems: which functions can panic, allocate or diverge | |
| 8 | refinement types | proving indexes in bounds, so indexing can't panic | |
| — | `alloc` (a dense, resizable `List`) and `std` | | after effects, once raw memory has a design |
| — | C backend | | when it's worth it |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
