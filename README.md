# T

Like C, but a different letter.

A small systems language with no garbage collector and no runtime, built one
type-theory concept at a time, mainly to learn what those concepts are and why
they matter.

```rust
enum Shape { Circle(int), Rect(int, int), Empty }

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
    print(fact(10));
    print(twice(fact, 3));
    print(twice(|x| x + n, 0));
    print(area(Shape::Rect(3, 4)));
    print(area(Shape::Circle(2)) + area(Shape::Empty));
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
t lsp                       # language server (see below)
```

From a checkout, use `cargo run -- <args>` instead, and `cargo test` to run
every program in `tests/t/`.

Pipeline: `lexer` → `parser` → `check` (type checker) → `interp` (tree-walking
interpreter). A real backend (emitting C) comes once the type system has
enough in it to be worth compiling.

## Editor support

`t lsp` is a Language Server Protocol server over stdio. It reports errors and
warnings as you type, and offers their fixes as quick fixes (code actions).

**Neovim (0.11+):** install
[t-lang.nvim](https://github.com/lkurcak/t-lang.nvim), e.g.
`vim.pack.add({ "https://github.com/lkurcak/t-lang.nvim" })`.

**Helix:** merge [editors/helix/languages.toml](editors/helix/languages.toml)
into `~/.config/helix/languages.toml`.

**Anything else with an LSP client:** run `t lsp` for files ending in `.t`.

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

**Neovim (0.11+):** build the parser and copy the queries into your
`site` directory, from a checkout:

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

```rust
print(1 + 2);   // expect: 3
// error: expected `int`, found `bool`
// warning: unreachable pattern
```

Every warning must be expected, and applying the warnings' fixes must not
change what the program does.

## Design constraints

T should be good enough to write an operating system in, so it has to
compile to bare metal: no garbage collector, and no runtime. Every feature
has to fit that:

- **Every value has a size known at compile time.** Generics are
  monomorphized, so there's no boxing and no type information at run time.
- **Nothing allocates behind your back.** The core language never touches
  the heap; allocation will live in a library, as in Rust's split between
  `core`, `alloc` and `std`.
- **Closures that capture are downward-only:** they can be passed to
  functions but not returned or stored, since their variables live on the
  stack. See [docs/03-inference.md](docs/03-inference.md).
- **Errors are values, not exceptions.** There's no unwinding.

Within that, the aim is to push the type system as far as it goes, and keep
it sound: well-typed programs don't go wrong, including the ones that
combine features in ways nobody planned for.

## Roadmap

| Step | Feature | Concept | Notes |
|---|---|---|---|
| 1 ✅ | ints, bools, `let`, functions, `if` | simply typed lambda calculus, soundness | [docs/01-stlc.md](docs/01-stlc.md) |
| 2 ✅ | structs, enums, `match` | algebraic data types, exhaustiveness | [docs/02-adts.md](docs/02-adts.md) |
| 3 ✅ | inference, closures | unification / bidirectional typing | [docs/03-inference.md](docs/03-inference.md) |
| 4 | generics | rank-1 (prenex) parametric polymorphism, monomorphization | |
| 5 | ownership, borrowing, pointers | affine types, regions/lifetimes; types that hold themselves through a pointer | |
| 6 | traits | ad-hoc polymorphism, typeclasses, coherence; closures as `Fn` traits | |
| 7 | effects | effect systems: which functions can panic, allocate or diverge | |
| 8 | refinement types | proving indexes in bounds, so indexing can't panic | |
| — | C backend | | when it's worth it |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
