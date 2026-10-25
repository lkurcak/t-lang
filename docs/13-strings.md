# Step 13: strings

Code: the literals in `src/lexer.rs` (`quoted`), their types in
`src/check/expr.rs` (T-Char, T-Str) and the borrow checker
(`src/borrowck/build.rs`), `str` as a type only behind `&` in
`src/check/generics.rs` (`resolve`) and `src/types.rs` (`unsized_part`),
`char` in the refinement logic in `src/check/refine.rs` (`scalar`) and in
`src/exhaust.rs`, and the library in `core/str.t` and `core/char.t`.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = "añb€";
    out.line(&s.len())?; // expect: 7
    for c in s.chars() {
        out.line(&(c as u32))?;
    }
    // expect: 97
    // expect: 241
    // expect: 98
    // expect: 8364
    match s.get(1, 3) {
        Option::Some(t) => out.line(&t)?, // expect: ñ
        Option::None => {},
    }
    Result::Ok(())
}
```

A string is bytes, `s.len()` counts them, and its characters are what
`chars` decodes from them: `ñ` is two bytes and `€` three. `get(1, 3)` is
bytes 1 to 3, which are a character; `get(1, 2)` would be half of one, so
it's `None`.

## `str`: bytes that are UTF-8

`str` is a type like a slice, `[u8]` (docs/10-slices.md): some number of
bytes, how many known only as the program runs, so never a value of its
own, only what a reference points to. It's one thing more: its bytes are
**UTF-8**. That's a promise each `&str` keeps, an invariant of the type,
like the ones a struct's fields keep in docs/08.1, but kept by a
different means: nothing can change a `str`'s bytes. So it's only ever
behind a *shared* reference, `&str`, never `&mut str`:

```rust,error
fn shout(s: &mut str) {}
// error: `str` can only be behind a shared reference, as in `&str`
```

What a `str` can do is in `core/str.t`, and almost all of it is safe code
over its bytes. Two things are trusted:

- `as_bytes(&self) -> &[u8]`, its bytes, lent to be read. It's the same
  reference; the type forgets the promise, which is always safe.
- `from_utf8_unchecked(b: &[u8]) -> &str with unsafe`, the other way: the
  type makes the promise, so its caller must keep it (docs/09-unsafe.md).
  The interpreter checks that it did:

```rust,error
fn main() {
    let b = b"a\xC3";
    // SAFETY: wrong: `\xC3` starts a character that never ends.
    let s = unsafe { core::str::from_utf8_unchecked(b) };
    let _v = s.len();
}
// error: unsafe code broke its obligation: `str::from_utf8_unchecked` of bytes that aren't UTF-8
```

The rest is written over those two. `from_utf8` checks the bytes and then
makes the promise, in an `unsafe` block that's right because it checked:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    match core::str::from_utf8(b"\xF0\x9F\x98\x80") {
        Option::Some(s) => out.line(&s)?, // expect: 😀
        Option::None => {},
    }
    match core::str::from_utf8(b"\xED\xA0\x80") {
        Option::Some(_) => out.line(&"UTF-8")?,
        Option::None => out.line(&"not UTF-8")?, // expect: not UTF-8
    }
    Result::Ok(())
}
```

The second is a surrogate (below), written as UTF-8 would write it, which
UTF-8 forbids. `split_at` and `get` cut a string only where a character
starts: UTF-8 cut there is UTF-8 on either side, the obligation of the
`unsafe` block in them. `chars` decodes. `==` compares bytes, and `Ord`
orders them, the first that differ deciding: UTF-8 was made so that's the
order of the characters' numbers.

## `char`: a Unicode scalar value

A `char` is a number from 0 to `D7FF`, or from `E000` to `10FFFF`: a
*Unicode scalar value*. The numbers in between, the *surrogates*, aren't
characters: UTF-16 writes a character past `FFFF` as two of them, and they
mean nothing alone. Like an integer type, `char` is a range of ℤ (docs/11),
but one with a gap in it, and with no arithmetic: `'a' + 1` is an error.
`as` converts, by the rule integers have: the same number, and a panic if
it isn't one of the type's.

```rust
fn next(c: char) -> char with panic { (c as u32 + 1) as char }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(next('a') as u32))?;        // expect: 98
    out.line(&('\u{10FFFF}' as u32 + 1))?; // expect: 1114112
    Result::Ok(())
}
```

`c as u32` never panics, as every character's number is a `u32`; nor does
`c as u32 + 1`, as the greatest is `10FFFF`. `(..) as char` may: `next` of
`'\u{D7FF}'` would be a surrogate. As with any `as`, what's proved can't
panic needs no `with panic`, so `char::from_u32` is safe code with no
intrinsic:

```rust
fn from_u32(n: u32) -> Option<char> {
    if n <= 0x10FFFF && (n < 0xD800 || n > 0xDFFF) { Option::Some(n as char) } else { Option::None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    match from_u32(0xDFFF) {
        Option::Some(_) => out.line(&1u8)?,
        Option::None => out.line(&0u8)?, // expect: 0
    }
    Result::Ok(())
}
```

The refinement logic knows a character's value lies in `0..=0x10FFFF`, the
interval its variable's kind says (docs/08), and, from where it turns into
a number, that it's no surrogate: a disjunction, which a kind, an interval,
can't hold.

Patterns match characters and ranges of them, and a `match` is exhaustive
when it covers both halves:

```rust
fn kind(c: char) -> u8 {
    match c {
        'a'..='z' | 'A'..='Z' => 1,
        '0'..='9' => 2,
        '\0'..='/' | ':'..='@' | '['..='`' | '{'..='\u{D7FF}' => 3,
        '\u{E000}'..='\u{10FFFF}' => 4,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&kind('7'))?; // expect: 2
    Result::Ok(())
}
```

## Literals, and what each escape means

| Literal | Type |
|---|---|
| `"text"` | `&'static str` |
| `'a'` | `char` |
| `b"bytes"` | `&'static [u8]` |
| `b'a'` | `u8` |

A literal's bytes are part of the program, there as long as it runs and
never changed, so a string literal is a shared reference that lives as
long as anything: `'static`. A line break may be written as it is, in a
string.

`\n`, `\r`, `\t`, `\\`, `\0`, `\'` and `\"` mean what they do in Rust. The
other escapes each mean one thing:

- `\u{20AC}` is a **character**, the one numbered `20AC`, written as its
  UTF-8 in a string. It's only in text, `"…"` and `'…'`, and must be a
  scalar value.
- `\xFF` is a **byte**. It's only in byte literals, `b"…"` and `b'…'`,
  where any byte can be written, and a character other than ASCII can't
  (`é` is two bytes: which did you mean?).

```text
"\x41"     error: `\x` escapes are for byte literals; write a character as `\u{..}`
b"\u{41}"  error: `\u{..}` escapes are for characters; write a byte as `\x..`
'\u{D800}' error: `\u{D800}` isn't a Unicode scalar value
```

Rust allows `\x` in strings too, up to `7F`, where it means the same as
`\u`: two spellings of one thing. And `\u` in a byte literal would have to
mean either a byte or a character's UTF-8; either reading would surprise
someone.

`'a'` is a character and `'a` a lifetime: a `'`, one character and a `'`
is a character literal, as in Rust.

## Panics say why

`panic` takes a message, and `Option` and `Result` have `expect`, `unwrap`
with one:

```rust,error
fn main() with panic {
    let o: Option<i64> = Option::None;
    o.expect("o has a value");
}
// error: runtime error: panicked: o has a value
```

A program writes a string to its output with `out.line(&s)?`, through
std's `Stdout` (since step 19, docs/19-std-io.md).

## Why it's sound

**Lemma S1 (strings are UTF-8).** Every `&str` value's bytes are UTF-8. A
`&str` is made only by a literal, whose bytes the lexer made from UTF-8
source and escapes of scalar values; by `from_utf8_unchecked`, whose
obligation it is; or from another `&str`, by safe code that can only lend
its bytes to be read, or by `unsafe` blocks in `core::str` that each keep
it. Nothing writes a `str`'s bytes: they're only ever borrowed shared
(there's no `&mut str`), and a literal's are in memory nothing writes. So
it holds by induction over the program's steps, for every safe program,
and every one whose `unsafe` blocks keep their obligations. The
interpreter checks it where it's promised.

**Lemma S2 (characters are scalar values).** Every `char` is a scalar
value: literals are checked by the lexer, and `as char` gives one or
panics (proved, or `with panic`), as Lemma I1 of docs/11 has it for
integers. So the logic may assume it of every character (its range, as a
kind, and the gap, as a fact), which `decode` in `core/str.t` relies on to
prove that it can't panic.

**Lemma S3 (literals live forever).** A literal's bytes are in memory
that's never freed, nor written, so a `&'static` reference to them never
dangles and never sees them change. The borrow checker gives a literal the
region `'static`.

**Lemma S4 (`str` is never a value).** As for slices (docs/10), `str` is a
type only behind `&`, never what a type parameter stands for, nor what a
variable is solved to (`Clash::Unsized`): so no value of unknown size is
ever moved, stored or dropped. The one place this is relaxed is that the
built-in libraries may implement a trait for `str`, as `core` does `Ord`.
Its methods with `Self` by value, `max` and `min`, need `Self: Discard`,
which `str` isn't, so they're never made for it.

Decoding is safe code: where a `str`'s bytes being UTF-8 makes a case
impossible, the code still handles it (`chars` stops), so nothing trusts
the invariant but the `unsafe` blocks, each of which says why it's kept.

## Design decisions worth noticing

- **`str` is built in, not a struct.** A struct with an unsized field
  (`struct Str { bytes: [u8] }`) would need values whose size is known only
  as the program runs, in general, which T doesn't have. As a type of its
  own, `str` is only what slices already are, plus the promise.
- **Two trusted operations.** Validation, decoding, cutting and ordering
  are T code, checked like any other, rather than intrinsics.
- **`char` is a type, not a refined `u32`.** It needs literals, patterns
  and exhaustiveness of its own, and no arithmetic; what's refined about it
  is only how the logic sees it.
- **One meaning per escape.** `\u` for characters, `\x` for bytes.
- **`panic` always says why.** There's no `panic()` without a message.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `String`, an owned, growing string | a `List<u8>` that keeps the promise; alloc | 13.1 |
| `Box<str>`, `Box<[T]>` | values whose size is known only as the program runs, owned | — |
| `s[i]`, `&s[1..3]` | indexing, with proofs about lengths (measures): `s.as_bytes()[i]` is 21 ✅; a range of a string, `&s[1..3]`, needs proofs of where characters start | — |
| string patterns, `"a" => ..` | they'd compare with `==`, which a pattern can't call yet | — |
| `<` on strings and characters | `<` is for integers; `Ord` orders strings, and `as u32` characters | 17.1 |
| `starts_with`, `find`, `split`, `trim`, `lines` | safe code over `as_bytes`, any time | — |
| raw strings, `r"…"`, and `\` before a line break | nothing needs them yet | — |
| `print_str` going away | std I/O | 19 |

## Exercises

1. Write `fn starts_with(s: &str, p: &str) -> bool` over `as_bytes`. Does
   it need `unsafe`? Then `strip_prefix`, which gives what's after: does
   that?
2. `decode` puts a character's bits together with `+`, not `|`. Change one
   to `|` and time `t check`. Why is it slower? (The logic's facts about
   `|` are cases.)
3. Why may `split_at` give two `&'a str`s of the same `'a`, when
   `split_at_mut` on slices must give two disjoint `&mut`s?
4. Write `fn is_ascii(s: &str) -> bool`. Why can't `as_bytes` be given a
   `&mut [u8]` version, `as_bytes_mut`, safely, though `make_ascii_uppercase`
   could be written in `core`?
5. Why is it sound for `Ord for str` to be an impl for an unsized type,
   though a trait's methods could take `Self` by value? What would break if
   `str` were `Discard`?
6. What would go wrong if a character literal could be a surrogate, as
   `'\u{D800}'`? Find the lemma whose proof breaks.

## Reading

- Pike and Thompson, *Hello World, or Καλημέρα κόσμε, or こんにちは 世界*
  (1993): UTF-8, by the people who designed it.
- The Unicode Standard, chapter 3, *Conformance*: scalar values, and the
  definition of well-formed UTF-8 (table 3-7), which `decode` follows.
- RFC 3629, *UTF-8, a transformation format of ISO 10646* (2003).
- The Rust reference, *Tokens*, on literals and escapes, and the
  documentation of `str::from_utf8_unchecked`, whose obligation is the one
  here.
- Jung et al., *RustBelt* (POPL 2018): a type's *safety invariant*, which
  unsafe code may rely on, as `str` relies on UTF-8.
