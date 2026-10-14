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

fn adder(n: int) -> fn(int) -> int { |x| x + n }

fn main() {
    print(fact(10));
    print(twice(fact, 3));
    print(twice(adder(5), 0));
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

## Tests

Each `tests/t/*.t` file is a program with its expectations in comments:

```rust
print(1 + 2);   // expect: 3
// error: expected `int`, found `bool`
// warning: unreachable pattern
```

Every warning must be expected, and applying the warnings' fixes must not
change what the program does.

## Roadmap

| Step | Feature | Concept | Notes |
|---|---|---|---|
| 1 ✅ | ints, bools, `let`, functions, `if` | simply typed lambda calculus, soundness | [docs/01-stlc.md](docs/01-stlc.md) |
| 2 ✅ | structs, enums, `match` | algebraic data types, exhaustiveness | [docs/02-adts.md](docs/02-adts.md) |
| 3 ✅ | inference, closures | unification / bidirectional typing | [docs/03-inference.md](docs/03-inference.md) |
| 4 | generics | parametric polymorphism (System F), monomorphization | |
| 5 | recursive types | iso- vs equi-recursive types | |
| 6 | traits | ad-hoc polymorphism, typeclasses, coherence | |
| 7 | ownership, borrowing | affine types, regions/lifetimes | |
| — | C backend | | when it's worth it |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
