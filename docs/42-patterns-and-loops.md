# Step 42: string patterns, `for` over temporaries, `{self}`

Code: `PatternKind::Str` in `src/ast.rs`, typed in
`Checker::pattern` (`src/check/patterns.rs`), `Ctor::Str` in
`src/exhaust.rs`, `pattern_implies` in `src/check/refine.rs`, `Pat::Str`
in `src/mir.rs` with its test in `src/interp.rs` and `src/cgen.rs`
(`t_bytes_eq` in `src/cgen/shim.h`); `for_loop` in
`src/check/modules.rs`; `Leaf::module_only` there too; `assert` in
`core/panic.t`.

Writing a nonogram game in T (the t-nonograms repository) gave a list of
small things that got in the way. This step does the ones that are about
the language rather than the libraries: matching on strings, looping over
what a call or a const gives, importing a module with what's in it, and
indexing after an `assert`. Each is small; none is a new idea. What's
worth reading is why each is sound, and in one case why the obvious
version wouldn't be.

```rust
#[derive(Copy, Discard)]
enum Command { Help, Fill, Quit, Unknown }

const WORDS: [&str; 3] = ["help", "fill", "quit"];

fn command(word: &str) -> Command {
    match word {
        "help" | "h" | "?" => Command::Help,
        "fill" | "f" => Command::Fill,
        "quit" | "q" => Command::Quit,
        _ => Command::Unknown,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for w in WORDS.iter() {
        match command(*w) {
            Command::Unknown => out.line(&"?")?,
            _ => out.line(w)?,
        }
    }
    Result::Ok(())
}
// expect: help
// expect: fill
// expect: quit
```

## String patterns

A string literal is a pattern. It matches a `&str` with the same bytes,
anywhere a pattern goes: as an arm, an alternative, inside a variant, in
`if let` and `let .. else`.

```rust
fn yes(answer: Option<&str>) -> bool {
    let Option::Some("y" | "yes") = answer else { return false };
    true
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&yes(Option::Some("yes")))?;
    out.line(&yes(Option::Some("no")))?;
    out.line(&yes(Option::None))?;
    Result::Ok(())
}
// expect: true
// expect: false
// expect: false
```

Its type is the literal's, `&'static str`. What it's matched against may
be a `&'a str` of any `'a`, borrowed from anywhere: a `&'static str` is a
subtype of it, so the pattern constrains no region. Like every pattern
that looks inside, it looks through references (match ergonomics), but
only down to the last `&`: a `&&str`, as `iter` gives over a `[&str]`,
matches too.

It binds nothing and moves nothing. It reads the bytes the reference
points to, through a shared reference, as an integer pattern reads an
integer. The borrow checker sees the read (an `inspect` of the place, in
MIR), so nothing can change the string while it's matched, which nothing
could anyway.

A `match` on strings needs a `_`: there are as many strings as you like,
and no set of literals covers them all.

```rust,error
fn yes(s: &str) -> bool {
    match s {
        "y" | "yes" => true,
        "n" | "no" => false,
    }
}
// error: non-exhaustive `match`: `_` not covered
```

The exhaustiveness checker knows each literal as a constructor of its
own, `Ctor::Str`, equal to another exactly when the bytes are, so the
same string twice is an unreachable pattern, as `1` twice would be. And
`str` has no list of all its constructors, the case the checker already
had for slices and type parameters, so only `_` or a variable covers it.

A `String` isn't a `&str`: match on `s.as_str()`. A byte string, `b"x"`,
isn't a pattern yet.

### What a match tells the checker

Matching `"abc"` says the string has three bytes, so its bytes can be
indexed below three with nothing to check:

```rust
fn last_of_three(s: &str) -> u8 {
    match s {
        "abc" | "xyz" => s.as_bytes()[2],
        _ => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&last_of_three("xyz"))?;
    Result::Ok(())
}
// expect: 122
```

Here is the case where the obvious rule would be wrong. For integers, the
refinement logic (docs/08) is told *exactly* when a pattern matches: `5`
matches `x` iff `x == 5`. That's what lets the arms after it assume
`x != 5`. For a string, `s.len() == 3` is only *implied* by matching
`"abc"`: `"xyz"` has three bytes too, and doesn't match. Assuming
`s.len() != 3` in the arms after `"abc"` would be false of `"xyz"`, so the
checker would accept indexing that's out of bounds:

```rust,error
fn first(s: &str) -> u8 {
    match s {
        "" => 0,
        _ => s.as_bytes()[0],
    }
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

Had `""`'s length been negated, the `_` arm would know `s.len() != 0` and
this would check, and be right. But by the same rule, `"ab" => ..`
followed by `_ => ..` would let `_` assume `s.len() != 2`, which `"cd"`
breaks. So a pattern now gives the checker two things: what's equivalent
to its matching (`pattern_formula`, as before), assumed in its arm and
negated in the ones after, and what its matching only implies
(`pattern_implies`), assumed in its arm and nowhere else. An
alternative implies only what one of its alternatives does: `"abc" |
"xy"` gives `s.len() == 3 || s.len() == 2`.

**Lemma S1 (string patterns).** A string pattern of bytes `b` matches a
`&str` `s` iff `s` points to exactly the bytes `b`. Then (i) matching
reads `s`'s bytes and nothing else, so it's sound for the borrow checker
to treat it as an `inspect` of `*s`; (ii) `s.len()` is the number of bytes
`s` points to (a measure, docs/21), so a match implies `s.len() ==
b.len()`, while a failed match implies nothing about `s.len()`; (iii)
`str` has infinitely many values and the literals of one `match` are
finitely many, so they never cover it, and requiring a `_` is exact;
(iv) two literals match the same strings iff their bytes are equal, so
`Ctor::Str` by bytes is exact for redundancy.

The interpreter compares the bytes the reference points to; C calls
`t_bytes_eq`, in the part of the runtime that needs no C library, so
string patterns work on the freestanding target (docs/39).

## `for` over a temporary

Until now, a `for`'s head couldn't borrow from what it made:

```rust
const WORDS: [&str; 2] = ["a", "b"];

fn squares() -> [u32; 3] { [1, 4, 9] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for w in WORDS.iter() {
        out.line(w)?;
    }
    for x in squares().iter() {
        out.line(x)?;
    }
    Result::Ok(())
}
// expect: a
// expect: b
// expect: 1
// expect: 4
// expect: 9
```

was "a temporary value is dropped here while still borrowed". A use of a
const is a new value each time (docs/36), a call's result is one, and
`.iter()` borrows it; the rewrite of `for` into a `loop` (lemma C4,
docs/06e-core.md) began with `let mut iter = it;`, and a `let` drops its
initializer's temporaries at its `;`, before the loop runs.

Now `for p in it { body }` means

```text
match it {
    mut iter => loop {
        match core::iter::Iterator::next(&mut iter) {
            core::option::Option::Some(p) => body,
            core::option::Option::None => break,
        }
    },
}
```

which is Rust's rewrite too. A `match`'s scrutinee is part of the
statement the `match` is in, and a statement's temporaries live to its end
(docs/05), so what `it` makes lives until after the loop, and after
`iter`, which is dropped at the end of the arm, before them.

**Lemma F1 (`for` over temporaries).** The rewrite is a program anyone
could write, and every check runs on it after the rewrite. So it's sound
because `match` and `loop` are; nothing needed a new rule. In particular,
a reference into a temporary of the head still can't outlive the
statement:

```rust,error
fn squares() -> [u32; 3] { [1, 4, 9] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let zero = 0u32;
    let mut last = &zero;
    for x in squares().iter() {
        last = x;
    }
    out.line(last)?;
    Result::Ok(())
}
// error: a temporary value is dropped here while still borrowed
```

## `use a::{self, B}`

In a group, `self` names the module the group is in:

```rust
mod line {
    pub fn width() -> u8 { 5 }
    pub fn height() -> u8 { 4 }
}

mod game {
    use crate::line::{self, height};

    pub fn area() -> u8 { line::width() * height() }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&game::area())?;
    Result::Ok(())
}
// expect: 20
```

It brings in what `use crate::line;` would, the module, and only that: if
`crate` also had a function named `line`, it wouldn't come along, as it
would with `use crate::line;`. `{self as l}` names it `l`. Outside a
group, `use crate::line::self;` is still an error, as in Rust: it says
nothing `use crate::line;` doesn't. `use crate::{self}` is an error too:
the crate root has no name to bring in. Under an enum, since step 56, it
names the enum, whose variants the rest of the group are
(docs/56-field-paths-and-slices.md).

While here: `use crate::line;` in the module that already has `line` used
to say "these imports go round in a circle". It said that because looking
for `line` there found the `use` itself. Now it says `line` is already a
name in this module.

## `assert` ensures what it checks

```rust
fn third<'a>(s: &[&'a str]) -> &'a str {
    assert(s.len() == 3);
    s[2]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&third(["f", "3", "2-5"].as_slice()))?;
    Result::Ok(())
}
// expect: 2-5
```

`assert(ok: bool) with panic ensures ok`: once it returns, `ok` holds, and
the caller's checker knows it (`ensures`, docs/30). Nothing is trusted:
`assert`'s body proves it, as it returns only past `if !ok { panic(..) }`.
`assert_eq` ensures nothing yet: its values are of any `T: Eq`, and the
logic knows nothing of what `==` means for one.

## Design decisions worth noticing

- **A string pattern is a test of its own, not a call to `==`.** Rust
  matches strings the same way. A pattern that called `Eq::eq` would run
  user code inside a `match`, with its effects, and the exhaustiveness
  checker couldn't know two impls agree.
- **Implied facts beside exact ones.** The refinement logic had only
  "iff" facts for patterns. Strings are the first pattern whose matching
  implies a fact without being equivalent to it; keeping the two apart is
  what stops the wrong negation above.
- **`for` as Rust writes it.** Extending the life of temporaries in `let`
  for method receivers would have fixed `for` too, but changed every
  `let`; the `match` form changes nothing else.
- **`self` only in groups.** The one form that names something no other
  form can say as briefly.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| byte string patterns, `b"GET"` | no program has needed them; the same test on `&[u8]` | — |
| `assert_eq` ensuring `left == right` | the logic has no `==` for any `T` | — |
| a `const` of `&[&str]` | needs promoting what it points to into the program's data | — |
| string patterns on `String` | match on `s.as_str()`; deref patterns are a bigger feature | — |

## Exercises

1. Make `pattern_formula` give `s.len() == 3` for `"abc"`. Write the
   shortest program the checker then accepts that indexes out of bounds.
2. Why may a string pattern's type be `&'static str` when the string it's
   matched against is a `&'a str` that lives only in one function? What
   would go wrong if it were `&'a str` with a new region?
3. Write the `for` rewrite with `let` in place of `match`, as before, but
   with the head's temporaries kept: what would `let` need?
4. `"é"` and `"e\u{301}"` look the same. Does `match` say they're equal?
   Should it?
5. Use `match` on strings to write a command parser that takes `f 3 2-5`.
   Which parts still need byte indexes?

## Reading

- The Rust Reference, *Temporary scopes* and *`for` iterator loops*: the
  rewrite this step adopts, and why a `match` keeps its scrutinee's
  temporaries.
- Luc Maranget, *Warnings for pattern matching* (JFP 2007): the
  usefulness algorithm in `src/exhaust.rs`, where a type with infinitely
  many constructors, like `str`, is the default-matrix case.
- Niki Vazou et al., *Refinement Types for Haskell* (ICFP 2014): measures
  of data, and why a fact implied by a match isn't the same as the match.
