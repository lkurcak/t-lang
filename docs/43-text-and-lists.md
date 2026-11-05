# Step 43: text and lists in the libraries

Code: `Pattern`, `Split`, `SplitWhitespace`, `Lines`, `Parse` and the
new `str` methods in `core/str.t`; the integers' `Parse` and
`ParseIntError` in `core/num.t`; `char::is_whitespace` and `to_digit` in
`core/char.t`; `Ord for [T]` and `[T]::contains` in `core/slice.t`;
`Option<&T>::copied` in `core/option.t`; `Eq`, `Ord`, `first`, `last`
and `contains` for `List` in `alloc/list.t`. In the checker,
`ImplDef::local_eqs` (`src/traits.rs`), the library-only trait impls for
`[T]` in `collect_impls` (`src/check/traits/collect.rs`), and a method's
type arguments in `Walker::expr` (`src/check/names.rs`).

The nonogram game (the t-nonograms repository) spent 90 of its lines
reading `f 3 2-5`: splitting at whitespace, splitting at commas, and
turning digits into numbers, all over byte indexes. Its friction log
asked for what any program reading text wants. This step adds that to
`core` and `alloc`, as T code the checker checks like any other: no new
intrinsic, no new `unsafe`, and every function total, so none of it
adds `panic` to a caller's effects.

```rust
fn span(s: &str) -> Option<u32> {
    let mut total = 0u32;
    for part in s.split(',') {
        let n: u32 = part.trim().parse().ok()?;
        total = checked_add(total, n)?;
    }
    Option::Some(total)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for line in "f 3 2-5\nclue 1, 2, 3\n".lines() {
        let mut words = line.split_whitespace();
        match words.next() {
            Option::Some("clue") => {
                let rest = line.strip_prefix("clue").unwrap_or("");
                out.line(&span(rest).unwrap_or(0))?;
            },
            Option::Some(w) => out.line(&w)?,
            Option::None => {},
        }
    }
    Result::Ok(())
}
// expect: f
// expect: 6
```

## What was already there

The log said `Eq` was missing for slices and `Option`. It wasn't: `==`
compares slices, arrays, `Option`s and `Result`s whose values compare.
What was missing was `Eq` for
`List`, which is now its slice's. What `assert_eq(&o, &Option::Some(42))`
really lacks is a way to print an `Option`: `assert_eq` needs `Display`,
and `Display` is for what a program shows its user. That's a `Debug`
trait, and it's the next step.

## Patterns: what `str` looks for

`split`, `find`, `contains`, `starts_with`, `ends_with`, `strip_prefix`
and `strip_suffix` take a `Pattern`: a `char` or a `&str`.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"a→b".find('→').unwrap_or(99))?;
    out.line(&"--name".strip_prefix("--").unwrap_or("?"))?;
    for w in "a,,b,".split(',') {
        out.line(&w.len())?;
    }
    Result::Ok(())
}
// expect: 1
// expect: name
// expect: 1
// expect: 0
// expect: 1
// expect: 0
```

`Pattern` has three methods: where it's first found from a byte on, and
how many bytes of a string it is if the string starts, or ends, with it.
A `&str` is found where its bytes are, and a `char` where its UTF-8 is.

The trait is *sealed* (docs/29-closed-and-sealed-traits.md): only `core`
implements it. Nothing unsound would follow from a wrong impl, as every
cut goes through `str::split_at`, which checks it's at a character
boundary. But the methods would quietly give wrong pieces, and sealing it
keeps the trait free to change; patterns that are closures, `|c| c ==
',' || c == ';'`, are the obvious next impl. A sealed trait can be
unsealed later, and an open one can't be sealed.

### Why a `&str` is found where characters start

A string's bytes are UTF-8, and so are the pattern's. In UTF-8, a
character's first byte is never one that continues another
(`10xxxxxx`), so where a non-empty pattern's bytes match, its first byte
starts a character of the string, and the byte after its last one starts
the next character, or is the end. The empty string matches anywhere, and
`split` and `find` only look for it where characters start. That's why
cutting the string there, with `split_at`, always succeeds. The code
still handles `split_at` giving `None`, with an empty piece, as it has no
proof of the argument above; a refinement that said "a character starts
here" would let it.

### `split`, exactly

`split` gives the pieces between the places the pattern is found, front
to back, the places not overlapping: `"aaa".split("aa")` is `""` then
`"a"`. A string with n places gives n + 1 pieces, so `"".split(',')` is
one piece, `""`, and `",".split(',')` two. These are Rust's rules; the
empty pattern is found before each character and at the end, so
`"ab".split("")` is `""`, `"a"`, `"b"`, `""`.

The pieces are `&'a str` of the string being split, never of the
pattern: `Split<'a, P>` holds the pattern by value, and its `Item` is
`&'a str`. So a pattern made for the call can be gone before the pieces
are used, and the pieces can't outlive the string:

```rust,error
use alloc::string::String;

fn main() with alloc {
    let first: &str;
    {
        let s = String::from("a,b");
        first = s.as_str().split(',').next().unwrap_or("");
    }
    let _ = first.len();
}
// error: `s` doesn't live long enough
```

`split_whitespace` and `lines` are what reading commands and files
want: words between runs of whitespace, none empty; and lines without
their `\n` or `\r\n`, with no empty last line after a final `\n`.
Whitespace is Unicode's `White_Space` property, which
`char::is_whitespace` gives: the ASCII ones, and the no-break and
ideographic spaces and line separators beyond.

## `Parse`: reading values from text

```rust
use core::num::ParseIntError;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"-128".parse::<i8>().unwrap_or(0))?;
    match "256".parse::<u8>() {
        Result::Ok(n) => out.line(&n)?,
        Result::Err(e) => out.line(&e)?,
    }
    out.line(&("+7".parse::<u32>() == Result::Ok(7)))?;
    out.line(&("-0".parse::<u32>() == Result::Err(ParseIntError::InvalidDigit)))?;
    Result::Ok(())
}
// expect: -128
// expect: number too large to fit in target type
// expect: true
// expect: true
```

`Parse` is Rust's `FromStr`: a type says what's wrong with a string that
isn't one of it, `type Error`, and how to read one, `fn parse(s: &str) ->
Result<Self, Self::Error>`. `s.parse::<T>()` calls it. The integers,
`bool` and `char` implement it; a program's own types can too. It isn't
in the prelude, as Rust's isn't: `s.parse()` needs no import, and
`u32::parse(s)` needs `use core::str::Parse;`.

The integers' impl is one blanket impl for every `T: Int`, as `Display`'s
is. It reads digits most significant first, with `checked_mul` and
`checked_add`, or `checked_sub` after a `-`, so that the least value of a
signed type, `-128` for `i8`, is read without passing through `128`,
which doesn't fit. Overflow is an error value, not a panic.

### A fix this needed: an impl knows its own associated types

The blanket impl says `type Error = ParseIntError` and then returns
`Result<T, ParseIntError>`. The checker refused: the trait's signature
says `Result<Self, Self::Error>`, with `Self` the type parameter `T`, and
the projection `T::Error` didn't normalise, as for a type parameter it
only does through a bound. Inside an impl, though, the impl itself says
what it is.

`ImplDef::local_eqs` now adds `<Self as Trait>::A = ..` for each of the
impl's associated types, with `Self` its type, to what its methods may
assume, in their signatures and their bodies. That's sound because an
impl's methods are checked assuming its bounds, and at any type
arguments that meet them, coherence (no two impls overlap) makes it the
impl monomorphisation picks for its type; so after monomorphisation,
the projection normalises to the same type. It says nothing about other
types: inside `impl<T: Tr> Tr for Wrap<T>`, `T::Out` stays rigid.

### And two smaller ones

- **`impl Ord for [T]`.** As `str` has `Ord` and `&str` has `Ord`,
  slices now have both, so `a.cmp(b)` on two `&[T]`s finds the method,
  as it does on two `&str`s. Trait impls for `[T]` itself are allowed
  only in the built-in libraries, as for `str`: an impl must take care
  that no method takes `Self` by value, which `Ord`'s don't, but for
  `max` and `min`, which need `Self: Discard`, which a slice never is.
- **A method's type arguments may name the program's types.**
  `s.parse::<Point>()` said "unknown type `Point`": the pass that finds
  what each type name means walked a function's `::<..>` but not a
  method's.

## Lists and options

`List` gets `first`, `last` and, for `T: Eq`, `contains`, as its slice
has them; and `Eq` and `Ord`, as its slice's. `List` doesn't dereference
to a slice: T has no `Deref` and no implicit conversions, so method
lookup stays a function of the receiver's type, and `as_slice()` says
where a list is lent as a slice.

`Option<&T>::copied` gives an `Option<T>`, for a `T: Copy`, which the
game wrote a dozen times as a `match`.

`char::to_digit(radix)` takes a radix the checker proves is from 2 to
36, rather than panicking otherwise as Rust's does.

## Design decisions worth noticing

- **All in T.** Nothing here is an intrinsic. The libraries are where a
  language's claims about its checker get tested: writing these 540
  lines found the two checker gaps above.
- **Total.** Every function here has no effects. Indexing is proved
  (`bytes_at` takes an `at` with `at + n.len() <= h.len()`), and where a
  proof is out of the checker's reach, the code handles the case it
  can't rule out, rather than saying `with panic`.
- **Rust's names and rules.** Where Rust's behaviour is a choice
  (`split` on an empty string, what `trim` trims, `+7`), T makes the
  same one. Someone who knows one knows the other.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `Debug`, `#[derive(Debug)]`, `assert_eq` on `Option` | `Display` is for users; a second trait is needed | 43.1 |
| closures as patterns, `split(|c| ..)` | the sealed trait leaves room; no program has needed it yet | — |
| `rsplit`, `splitn`, `char_indices`, `rfind` | add as programs ask | — |
| `out.line("x")` without `&` | needs type parameters that stand for `str`: a language question | retro 4 |
| `parse` in other radixes | `to_digit` has the radix; `from_str_radix` when a program wants it | — |
| `Deref` from `List` to slices | implicit conversions; `as_slice()` is explicit | — |

## Exercises

1. Write `rsplit_once(s, c)`: the pieces before and after the last `c`.
   Which of `Pattern`'s methods would you add to `core` to write it
   without scanning twice?
2. Make `Pattern` unsealed and write an impl whose `find_in` gives
   `start` past `end`. What does `split` give? Is anything unsound?
3. Why does the integers' `parse` subtract digits after a `-` rather than
   add them and negate at the end?
4. `ImplDef::local_eqs` adds equalities for the impl's own type only.
   Write the program that would be unsound if it added
   `<T as Tr>::A = ..` for each type argument `T` of the impl's type too.
5. `"é"` is one `char` and `"e\u{301}"` two. What does `"e\u{301}".parse::<char>()`
   give, and what should a game's input reader do about it?

## Reading

- The Rust standard library, `core::str::pattern` and `FromStr`: the
  design this step follows, and the `Pattern` API that's still unstable
  there for the reasons it's sealed here.
- The Unicode Standard, *PropList.txt*: the `White_Space` property, and
  Annex #29 for why a `char` isn't what a reader sees as one character.
- The rustc dev guide, *Normalization in the new solver* and *Trait
  solving*: how Rust normalises a projection, and why coherence lets an
  impl's own associated types be assumed inside it.
