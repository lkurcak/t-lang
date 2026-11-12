# Step 70: an impl's bounds are its trait's

Code: `same_signature`, `bound_text` and `differs` in
`src/check/traits/impls.rs`; the generated family in
`tests/generated_bounds.rs`.

Retrospective 8 found that an impl's method could say more of its type
parameters than its trait's method did, as long as what it said was an
equality, `I: Iterator<Item = u8>`, a closure's effects, `F: FnMut() ->
u8 with {}`, or (since 0.83.0) an associated effect's bound, `I:
Iterator<e = {}>`. Its body was checked counting on that, and a call
through the trait, going by the trait's signature, never showed it held:

```rust,error
trait Run {
    fn run<I: Iterator + Discard>(i: I) -> u8 with I::e;
}

#[derive(Discard)]
struct R {}

impl Run for R {
    fn run<I: Iterator<Item = u8> + Discard>(mut i: I) -> u8 with I::e {
        match i.next() {
            Some(v) => v,
            None => 0,
        }
    }
}

fn via<X: Run>() -> u8 { X::run("ab".chars()) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&via::<R>())?;
    Ok(())
}
// error: its bound says `I: Iterator<Item = u8>`, and the trait's doesn't: a call through the trait wouldn't show it holds
```

0.84.0 accepted that, and `run` read each `char` of `"ab"` as a `u8`:
the interpreter stopped at the first comparison of one, as no `u8` is a
`char`, and compiling it to C failed in the C compiler, by luck, as the
two options have different C types. A closure's effects, since 0.8.0,
and an associated effect's bound, since 0.83.0, let a method that says
`with {}` panic:

```rust,error
trait Run {
    fn run<effect g, F: FnMut() -> u8 with g + Discard>(f: F) -> u8 with g;
}

#[derive(Discard)]
struct R {}

impl Run for R {
    fn run<effect g, F: FnMut() -> u8 with {} + Discard>(mut f: F) -> u8 with {} { f() }
}

fn boom() -> u8 with panic { panic("boom") }

fn via<X: Run>() -> u8 with panic { X::run(boom) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&via::<R>())?;
    Ok(())
}
// error: its bound says `F: FnMut() -> u8`, where the trait's says `F: FnMut() -> u8 with g`
```

## The same bounds

An impl's method's bounds are its trait's method's, at the impl's type:
its trait bounds, its lifetime bounds (as before), and now what its bounds
say an associated type is, and what they say an associated effect may be
at most. Up to names: its parameters, lifetimes and effect parameters can
be called anything, and an effect set written in any order.

```rust
#[derive(Discard)]
struct Up { n: u8 }

impl Iterator for Up {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> {
        if self.n < 3 {
            self.n = self.n + 1;
            Some(self.n)
        } else {
            None
        }
    }
}

trait Sum {
    fn sum<I: Iterator<Item = u8, e = {alloc, panic}> + Discard>(i: I) -> u8 with {alloc, panic};
}

#[derive(Discard)]
struct S {}

impl Sum for S {
    fn sum<K: Iterator<Item = u8, e = {panic, alloc}> + Discard>(mut k: K) -> u8
    with {panic, alloc}
    {
        let mut s: u8 = 0;
        while let Some(v) = k.next() {
            s = s + v;
        }
        s
    }
}

fn total<X: Sum>(_x: &X) -> u8 with {alloc, panic} { X::sum(Up { n: 0 }) }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&total(&S {}))?; // expect: 6
    Ok(())
}
```

A weaker bound is refused too, though it would be sound: an impl's method
that needs less than its trait's says could take what any call through the
trait gives it. Its signature would then have to be checked against the
trait's under the trait's bounds, by entailment rather than by
comparison; nothing has needed it.

The comparison takes both signatures apart whole, so a part added to
them later doesn't compile there until it's compared too: the two that
were missed were each added after the comparison was written, equalities
in step 6b and effect bounds in step 68.1.

## A const parameter is a parameter of its own kind

An impl's method's parameters line up with its trait's in order, each of
the same kind. Effect parameters and `unsized` ones were told apart; a
const parameter wasn't, so one in a type parameter's place was refused
for the types it made, with the trait's parameters renamed to the impl's:
"parameter 1 should be `[N; T]`". It's refused as what it is now:

```rust,error
trait K {
    fn f<const N: usize, T: Discard>(a: [T; N]) -> usize;
}

#[derive(Discard)]
struct R {}

impl K for R {
    fn f<T: Discard, const N: usize>(a: [T; N]) -> usize { N }
}

fn main() {}
// error: its `T` is a type parameter, where the trait's `N` is a const parameter
```

## What a match keeps

Step 69 found that switching off the value a `match` keeps of its
scrutinee for its patterns' bindings (`pinned`, step 65) changed no test,
and called it redundant (M12 of docs/69-loans.md). This step deleted it,
and t-libs's graph stopped checking: in `let Some(p) = parent[v] else
{ .. }`, the scrutinee is an element behind the reference `index` gives,
at no place the walk can name, so read again for the binding it says
nothing of what's declared there, `q < n`. What was matched does. It's
back, with a test of its own (`refine_let_else_index.t`): a rule no test
needs is one no test covers, not one nothing needs.

## Found along the way

The first program in the tests to call a trait's method through a type
parameter, `X::run(..)`, showed that renaming `X` in an editor left that
`X` as it was: a path's first segment was noted only when it named a
struct or an enum. It's noted when it names a type parameter too.

And `t fmt` ended a trait's method whose `where` clause went on lines of
its own with `,;`: the comma rustfmt puts after the last bound before a
body's `{` was put before the declaration's `;` too. It isn't now.

## Why it's sound

- **Lemma I1 (a call through a trait).** Code generic in `X: Tr` calls
  `X::m` going by `Tr`'s signature of `m`: it shows that the type
  arguments it gives meet `m`'s bounds as `Tr` says them, and nothing
  more. Monomorphisation then runs the impl's `m`, whose body was checked
  assuming the impl's bounds. So the impl's bounds must follow from the
  trait's, at the impl's type. Equal bounds do, and a part of a bound
  that isn't compared is a part that may not: each kind (trait,
  equality, associated effect, lifetime) is compared, each up to
  renaming (`Canon` for lifetimes, the substitution of the impl's
  parameters for the trait's, sorted sets for effects).
- **Lemma I2 (kinds).** A trait's parameter of one kind is given
  arguments of that kind by every caller: a type, an effect set, or a
  `usize` value. An impl's parameter of another kind in its place would
  be given what it can't take. So kinds line up, const included.

The effects' induction (docs/07) and the refinement checker's lemmas are
unchanged: they assumed what lemma I1 now makes true.

## Tested

`tests/generated_bounds.rs` builds the product of four kinds of bound (a
trait, an equality, an associated effect, a closure's effects), eight
positions (a function's own, a trait's method's and its impl's alike, an
impl's method stronger than its trait's, or weaker, an impl's `where`, an
opaque result's, a function made a function pointer, one called in a
closure), and an argument that breaks the bound or meets it: 64
programs. Each is refused or runs without the interpreter finding it
unsound; each breaking one is refused; each fitting one runs, but where
the position is refused whatever it's given. On 0.84.0, three broke: the
stronger impl, for an equality, an associated effect and a closure.

Each rule of this step switched off in turn makes a test fail, all
seven: comparing equalities (the family, `err_soundness_impl_method_item.t`,
`err_soundness_impl_method_closure_effect.t`), comparing effect bounds
(the family, `err_soundness_impl_method_assoc_effect.t`, and through a
dependency, `err_soundness_impl_method_dependency`), the const kind
(`err_impl_method_const_param.t`), ordering bounds before naming their
lifetimes (`impl_method_same_bounds.t`, a `where` clause written in
another order), noting a path's type parameter (the golden tests'
renaming), the comma before `;` (`tests/fmt.rs`), and the scrutinee a
match keeps (`refine_let_else_index.t`).

## What it doesn't do

- An impl's method can't need less than its trait's: its bounds are the
  trait's exactly (see above).
- Trait methods still can't have refinements or `ensures` (an impl's
  would have to ask no more and promise no less, as here).

## Decisions

- **Exactly the trait's bounds** (D1 of retrospective 8), as trait bounds
  already were: entailment is a later relaxation.
- **Taken apart whole**, so the next part of a signature can't be missed.
- **0.85.0.** Programs that were accepted are refused: each was unsound.

## Exercises

1. Write an impl whose method's `where` clause says `I::Item: Copy` where
   its trait's doesn't. Which kind of bound is it, and what does the
   checker say?
2. Lemma I1 asks the impl's bounds to follow from the trait's. Write a
   program that would be sound with a weaker impl bound, and say what the
   checker would have to prove to accept it.
3. Which other comparisons in the checker list a struct's fields by hand?
   What would each miss if a field were added?

## Reading

- Rust's E0276, "impl has stricter requirements than trait", and the
  `refining_impl_trait` lint: the same rule, with entailment.
- Philip Wadler and Stephen Blott, *How to make ad-hoc polymorphism less
  ad hoc* (POPL 1989): a call through a class goes by the class's
  signature, which each instance must meet.
