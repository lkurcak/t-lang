# Step 57: unsized parameters

Code: the kind in `Type::Param` and `Params::is_unsized` in
`src/types.rs`; the kinding rule in `resolve_in` in
`src/check/generics.rs`; `Clash::Unsized` and `unsized_ok` in
`src/check/unify.rs`, set by `fresh_param` in `src/check/effects.rs`;
`traits::matches` and coherence's `unify` in `src/traits.rs`; the
receiver rule in `method_call` in `src/check/expr.rs`; the kinds of an
impl's methods in `same_signature` in `src/check/traits/impls.rs`;
`sized` in `src/borrowck.rs`; the impls for references in `core/fmt.t`
and `core/cmp.t`.

A type parameter has stood for a type with a size, one whose values a
function can hold, move and drop without knowing more. `str` and slices
have none known when compiled (docs/10, docs/13), so no type parameter
could be one: `out.line("hello")` was how to write a line, as `line<D:
Display>(x: &D)` couldn't take a `&str` for `x`, only a `&&str`. And
`core` had two impls for each of `Display`, `Debug` and `Ord` on
references: `&'a T`'s, which couldn't cover `&'a str`, and `&'a str`'s
own.

A parameter declared `unsized` may stand for any type, those too:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line("hello")?; // expect: hello
    let s: &str = "a slice of it";
    out.line(s)?; // expect: a slice of it
    Ok(())
}
```

## `unsized` parameters

`unsized D` declares one, in the `<..>` of a function, a method, an
impl, a struct, an enum or a type alias. It may have bounds, as any type
parameter may:

```rust
use core::fmt::Display;

/// Any `Display`, `str` and slices too.
fn twice<unsized D: Display>(out: &mut std::io::Stdout, x: &D) -> Result<(), std::io::Error> {
    out.put(x)?;
    out.line(x)
}

/// A reference held in a struct: `Ref<'a, str>` holds a `&'a str`.
#[derive(Copy, Discard)]
struct Ref<'a, unsized T> { r: &'a T }

fn get<'a, unsized T>(r: Ref<'a, T>) -> &'a T { r.r }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    twice(&mut out, "ab")?;            // expect: abab
    twice(&mut out, &7u8)?;            // expect: 77
    out.line(get(Ref { r: "held" }))?; // expect: held
    Ok(())
}
```

The price is that a `D` is never a value. It can be only where `str` can:
right behind a reference, `&D` or `&mut D`; as the type argument for
another `unsized` parameter, as in `Ref<'a, D>`; as what a projection
is of, `D::Item`; and before the `:` of a bound, `where D: Display`.
Not a parameter's type, nor a field's, a `let`'s or what a function
gives, nor inside an `Option`, an array, a slice or a box:

```rust,error
fn f<unsized D>(x: &D) -> Option<D> { None }
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
```

And what a `&D` points to is a place, to borrow again or to call a
method of that takes `&self`, never a value, even if `D` is `Copy`: at
`D = str` it would be a value with no size.

```rust,error
fn f<unsized D: Copy>(x: &D) {
    let _y = *x;
}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
```

A method that takes `self` by value can't be called on one either, as
calling it would make that value.

## What stands for what

A type parameter that isn't `unsized` still has a size, so its function
may make values of it, as `copy` does here. It can't stand for an
`unsized` one, nor for `str` or a slice:

```rust,error
fn copy<T: Copy>(x: &T) -> T { *x }

fn f<unsized D: Copy>(x: &D) {
    let _y = copy(x);
}
// error: a type parameter that isn't `unsized` can't stand for `D`, which is
```

An `unsized` parameter may stand for any type, and its type variable may
be worked out to be any; written out, `twice::<str>(..)` is fine. Every
other type variable can't turn out to be `str`, a slice or an `unsized`
parameter, and a variable that may is made equal to one that can't, it
can't either.

## Impls

An impl may have `unsized` parameters as well. `core` now has one impl
of `Display` for references:

```text
impl<'a, unsized T: Display> Display for &'a T {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { (**self).fmt(w) }
}
```

which covers `&str` as `str`'s own impl does, and `Debug` and `Ord` for
`&'a T` alike, `&str` and `&[T]` among them. The six impls for
`&'a str`, `&'a [T]` and `&'a mut [T]` that were there because `T` couldn't
be either are gone.

Which impl a type uses goes by the impl's parameters' kinds: one that
isn't `unsized` matches only a type with a size. So `impl<'a, T:
Display> Tr for &'a T` is no impl for `&str`, nor for a `&D`, which may
be one; an `impl<'a, unsized U> Tr for &'a U` is, and it overlaps the
other, at `&i64`.

What an impl is for has a size, as a value's type does: `impl<unsized
T: Display> Tr for T` is an error. Implementing a trait for `str` or a
slice stays something only the built-in libraries do, taking care that
each method of the trait has `Self` behind a reference (lemma S4 of
docs/13-strings.md). Implement it for `&T` instead. A method of an impl
declares each parameter as the trait's method does, `unsized` or not.

## Why it's sound

Types have kinds: ★, those with a size, and ★?, any type. Every type of
kind ★ is of kind ★? too. `str`, `[T]` and an `unsized D` are of kind
★? only; `&T` and `&mut T` are of kind ★ for a `T` of kind ★?, and an
ADT's argument has the kind of its parameter.

- **Lemma U1 (kinding).** Every type written is well-kinded where it's
  written: a type in a position of kind ★, that of a value, is of kind
  ★ (`resolve_in`, with `unsized_ok` false there). The positions of kind
  ★? are those listed above.
- **Lemma U2 (substitution).** If `t` is well-kinded with `D` of kind
  ★?, and `u` is of kind ★?, then `t` with `u` for `D` is well-kinded:
  `D` is only in positions of kind ★?, where `u` may be. And with a
  parameter of kind ★, for one of kind ★, likewise. So a well-kinded
  function's instances are well-kinded, which is what monomorphisation
  makes.
- **Lemma U3 (inference).** A type variable for a parameter of kind ★
  is never solved to one of kind ★? only (`Clash::Unsized`), and when
  two variables are made equal, the one left may be unsized only if both
  could be. So every type argument inferred has the kind of its
  parameter, and U2 applies to it.
- **Lemma U4 (values).** No value has a type of kind ★? only. The
  checker keeps those types out of values as it goes: a deref of `&D` is
  a place, not a value; a method that takes `self` by value isn't called
  on a `D`. What a type variable turns out to be is known only once a
  function is checked, so once its MIR is built, every local (each
  parameter, variable and temporary: every value there is one) is
  checked to have a size (`sized` in `src/borrowck.rs`). With U2, no
  instance makes a value of unknown size: lemma S4 of docs/13 holds of
  generic code too, and monomorphisation checks that it does. (In T as it
  is, such a value would also need copying, from a type that's `Copy`,
  which no unsized type is, or moving out of a reference, which the
  borrow checker forbids; so the check is what keeps generic code
  well-kinded, not the last thing between a program and memory it can't
  size.)
- **Lemma U5 (impls).** Matching an impl's type against a type puts
  each of the impl's parameters for a part: one of kind ★ only for a
  part of kind ★ (`traits::matches`). So an impl found for a type is an
  instance of the impl at well-kinded arguments, and U2 makes its methods
  well-kinded there. Two impls overlap if some well-kinded type matches
  both: unifying their types, a parameter that isn't `unsized` may be
  the other impl's `unsized` parameter (both are then that sized type),
  but not `str`, a slice, or an `unsized` parameter of where the impl is
  used. Each parameter is in its impl's type once (impl headers are
  linear, docs/06a), so it meets nothing else that could need more of
  it.
- **Lemma U6 (no `&mut str`).** A `&mut D` at `D = str` is the type of
  no value. A `&mut str` could only be made by `&mut` of a place of type
  `str`, and every such place is `*r` of an `r: &str` (or of a
  `&mut str`, of which by induction there's none), which can't be
  borrowed mutably. No intrinsic gives a `&mut` of an `unsized`
  parameter. So `&mut D` is fine for generic code over `&mut [T]`, and
  no generic code can change a `str`'s bytes.
- **Lemma U7 (methods).** A trait's caller picks its methods' type
  arguments by the trait's kinds, so an impl's method declares the same
  ones: an `unsized` one in the trait that wasn't in the impl could be a
  value there, and `str` where it's called.
- **Parametricity.** What a function does with a `&D` doesn't depend on
  what `D` is: it can pass it on, borrow what it points to again, and
  call its bounds' methods. How big a `&D` is does: a pointer, or a
  pointer and a length at `str`. That's representation, as `T`'s size is
  for any `T`, and monomorphisation picks it per instance; a dictionary
  passing implementation would pass it as part of `D`'s dictionary.

The adversarial tests are `tests/t/err_soundness_unsized_*.t`: each is a
program that would make a value of unknown size, or pick an impl at an
ill-kinded type, if the rule it's named for were taken out.

## What it doesn't do

- No `Box<str>` or `Box<[T]>`: a box of something with no size known when
  compiled needs allocating with the size it has as the program runs.
- No user impls for unsized types: `impl Tr for str` stays the built-in
  libraries', and so does any impl whose type may be unsized. Allowing
  it would need trait default methods checked with `Self` maybe unsized,
  and a bound to say a method needs `Self` to have a size.
- No unsized associated types, and no trait type parameters, so no
  `Borrow<Q>` with an unsized `Q` for a map's lookups yet.
- No unsized last fields (`struct S { len: u8, rest: [u8] }`), nor
  `size_of_val`.
- A type argument with `_` inside it, as in `f::<Ref<'_, _>>`, has a `_`
  that must have a size; only a `_` that's the whole argument may be
  unsized.

## Decisions

- D1, where: in every generic declaration, functions, impls, structs,
  enums and type aliases, with one kind rule everywhere.
- D2, `&mut D` allowed, so that one impl covers `&mut [T]`; at `str`
  it's a type with no values (U6).
- D3, impls for possibly unsized types stay the libraries': an error for
  a program, rather than checking traits with `Self` maybe unsized.
- D4, programs drop the `&` of `line(&"..")`, which still works with
  `D = &str`.
- And: the kind is part of `Type::Param` (`Param(name, unsized)`), rather
  than looked up in a context. Every check that keeps `str` out of
  values asks `is_unsized`, which then sees an `unsized` parameter too,
  wherever the type has gone; and a parameter made with the wrong kind
  is a different type, which a test would show as a mismatch, rather than
  a quietly wrong answer.

## Exercises

1. Write `fn longer<'a, unsized T: Ord>(a: &'a T, b: &'a T) -> &'a T`.
   Call it with two `&str`s, two `&[u8]`s and two `&u8`s.
2. Take the `unsized_ok` condition out of `traits::matches`. Write a
   program that then makes an `Option<str>`, as
   `err_soundness_unsized_impl_select.t` would, and see what monomorphisation
   says.
3. Why does coherence's `unify` let a sized parameter be the other
   impl's `unsized` one, rather than refuse, as it does for `str`? Write
   the two impls that would otherwise both be found for `&i64`. Would
   it still be enough if an impl's type could name a parameter twice?
4. Remove `sized` from `analyze`. Which test now runs, and what does the
   interpreter say?
5. Why is `fn f<unsized D>(x: &mut D, y: &mut D) { core::mem::swap(x, y) }`
   rejected? What would go wrong at `D = [u8]`?

## Reading

- Grossman, *Quantified Types in an Imperative Language* (TOPLAS 2006),
  and Jim, Morrisett, Grossman, Hicks, Cheney and Wang, *Cyclone: A Safe
  Dialect of C* (2002): kinds for type variables of unknown size, only
  behind pointers.
- Rust RFC 490, *DST syntax* (2014), and RFC 546 (2014): `?Sized`, the
  opt-out bound, and `Self` in traits unsized by default; T's opt-in
  keyword is the same kind, the other way round.
- Pierce, *Types and Programming Languages* (2002), chapter 29: kinds,
  and kind-checking as type-checking one level up.
