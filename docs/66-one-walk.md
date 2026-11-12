# Step 66: one walk

Code: `src/check/refine/mir.rs` (`Walk::bind_ref`, `Walk::mut_refused`,
`Walk::way_into`, `Walk::keep_wholes`, `Walk::lvalue_mir`) and
`Walk::variant_decl` in `src/check/refine/targs.rs`.

Step 65 put a second refinement checker beside the walk over each body's
AST: the same rules over the MIR the borrow checker built, which had to
find the same errors and prove the same operators on every program there
is. Now it's the only one. The walk over the AST, about 2,300 lines that
found places one kind of expression at a time, is gone, and with it
`T_LANG_REFINE` and the harness that compared the two.

On MIR, a variant's field is a place like any other, and so is an
array's element a pattern takes out: so a `&mut` a pattern binds into one
is now what `&mut` of a place is, and what's written through it is
checked, where step 62 refused the binding outright.

```rust
#[derive(Copy, Discard)]
enum Slot<T> { Free, Held(T) }

fn age(s: &mut Slot<{ t: u8 | t < 100 }>) {
    match s {
        Slot::Held(t) => {
            if *t < 99 {
                *t = *t + 1;
            }
        },
        Slot::Free => {},
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s: Slot<{ t: u8 | t < 100 }> = Slot::Held(41);
    age(&mut s);
    match s {
        Slot::Held(t) => out.line(&t)?, // expect: 42
        Slot::Free => out.line(&0u8)?,
    }
    let free: Slot<u8> = Slot::Free;
    let _ = free;
    Ok(())
}
```

`t` is a `&mut u8` into the `Held` of what `s` points to, and what's
declared there is `t < 100`: so `*t = *t + 1` must be below 100, which it
is under the `if`. Without the `if`, it can't be proved, and that's the
error now, at the write, not at the pattern.

## Variants' fields and elements as places

A `&mut` made by a pattern, by match ergonomics, is a `&mut` of the place
the pattern took apart: the scrutinee, through the `*`s it looked
through, and the fields, variants' fields and elements down to the
variable. The walk treats it as it treats `let r = &mut p`:

- **What's declared there** (`lvalue_mir`): what the place's value must
  keep, at each position of its type. Through a field, it's what's
  declared of the struct at that field's positions; through a variant's
  field, what's declared of the enum's type arguments at that variant's
  field's positions (`variant_decl`); through an array's element, what's
  declared of its elements; through a `*`, what the reference says. The
  reference the pattern makes says that, and every write through it must
  have it.
- **What it can't be** (`mut_refused`): what an explicit `&mut` of the
  same place can't be. A part of a refined variable; a field a struct's
  refinement is about; and, new in this step, a part of a value whose
  refinement is about the whole of it (below), or what a struct field's
  own refinement says of a variant's field in it (`way_into`), as
  `&mut s.o` of that field can't be.

> **Lemma V1 (writes through a pattern's `&mut`).** Let `a` be bound by
> `&mut` to place `q` of a value `v`. Each write through `a` changes
> exactly the value at `q`, and what's at other positions of `v`'s type
> stays. What's declared at each position below `q` is checked of every
> write, as the reference says it (lemma W1 of docs/63-what-is-declared.md).
> What's declared of the value at a position above `q` is of a value the
> write changes. Above a variant's field, that's an enum's value, and a
> refinement says of an enum's value only which variant it is, by its
> measures (docs/59-lends-past-the-call.md): a write to a variant's field
> doesn't change which variant it is. Below that, up to what a reference
> points to, through fields and boxes, a refinement of the whole can
> change, and the binding is refused, unless nothing's declared of the
> whole. So every refinement of `v` holds after each write, and the place
> the pattern took apart, which may change through `a` while `a` lives,
> is read as unknown but for what's declared there (unstable, as before;
> since step 69, from the binding on: docs/69-loans.md).

So an or-pattern's alternatives bind references to different places, each
saying what's declared at its own place; where the alternatives meet, the
reference says what both do, as any value joined there.

## A hole: a field of a value refined whole

Writing the tests for these bindings found a hole that had been there
since 0.76.0, when refinements could be type arguments of anything. A
refinement can be about the whole of a value that's somewhere only a
type argument reaches: a tuple's element, a list's, an array's, what a
box holds, what a `&mut` points to:

```rust,error
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn f(o: &mut { s: S | s.a < s.b }) {
    (*o).a = 100;
}
// error: can't prove `s.a < s.b` for `(*o)`, once `(*o).a` is written
```

This was accepted, as were `t.1.a = 100` of a `(u8, { s: S | s.a < s.b
})`, `l[0].a = 100` of a list of them, `arr[0].a = 100` and `b.a = 100` of
a box of one, and `&mut (*o).a` given anywhere. What a write must keep is
what's declared of the place written, at each position of its type, and
what's declared of a field was what's declared of the struct at the
field's positions: what's declared of the struct itself, at no position
below it, was dropped. The interpreter stopped on each, once the caller
read what it had lent.

Now a write to a field keeps what's declared of each value it's in
(`keep_wholes`), from the struct the field is of, outward, through
fields and boxes, up to what a reference points to: each is checked as it
will be, the value before with the part written replaced. A `&mut` of a
part of such a value is refused, as one of a part of a refined variable
is: once the reference exists, nothing knows which writes through it are
writes to the whole.

> **Lemma V2 (a write keeps the wholes it's in).** Let a write of `x` to
> place `q` change, besides `q`, the values `v1, ..., vn` it's in, innermost
> first, up to what a reference points to (`levels`). A value at a position
> where a refinement is declared has it before the write (by induction on
> the program's steps, every place has what's declared of it whenever it's
> read). After the write, `v1` is its old value with the field written
> replaced by `x`, `v2` is its old value with the field `v1` is in
> replaced by that, and so on: what `keep_wholes` builds. Each must have
> what's declared of its whole, and is checked to. What's declared of a
> value above `vn`, behind a reference, is of a reference, which the write
> doesn't change; what's declared at positions of the `vi` other than the
> ones the write is in, of values the write doesn't change.

## Why one walk is enough

Step 65 argued the walk over MIR sound on its own terms (lemmas M1 to M3
of docs/65-refinements-on-mir.md): facts hold on every path to a block,
every write is one of MIR's few statements, each checked by the rule the
walk over the AST used for what it was lowered from. Those lemmas are now
the whole argument. Its parity with the walk over the AST was checked
over every program in the tests, the libraries and the programs using
`t`, and the comparison goes with the walk it compared against. What
stands in for it is mutation testing against the tests themselves: each
rule of the walk over MIR was switched off in turn, step 65's nineteen and
this step's nine, and each time a test failed. Four needed a new test to
fail: a whole refined outside a box, two more forms of bindings, and one
that only changed which variables an explanation lists as known.

## Decisions

- **Checked like `&mut` of the place.** The alternatives were keeping step
  62's refusals, or making the walk's own places follow variants' fields,
  so a write through such a reference updates what's known of the enum.
  That precision shows only once a place a pattern binds by `&mut` stops
  being unknown for the whole function, which is a rule of its own.
- **Refused where a struct field's own refinement reaches into the
  variant.** That refinement is the struct's, a fact in the logic about
  each variant, not something a reference can carry: so, as `&mut s.o`
  can't be made of such a field, nor can a binding into it.
- **0.80.0.** Programs step 62 refused are accepted now, and the hole's
  fix refuses programs 0.79 accepted.

## What it doesn't do

- A place a pattern binds by `&mut`, or a closure captures, is still
  unknown for the whole function. The borrow checker knows where each
  borrow ends, and the place could be known again there. (Done in step
  69: docs/69-loans.md.)
- A part of a value refined whole can't be borrowed `&mut` at all, even
  straight to a call that can't keep it, after which the whole could be
  proved again, as a refined variable lent whole is.
- Variants' fields can't be refined themselves, `enum Key { Char({ c: u8
  | c < 128 }) }`; that's next, with refinement aliases (done in step 67,
  docs/67-refinement-aliases.md).

## Exercises

1. Write a function taking `&mut (u8, { t: (u8, u8) | t.0 < t.1 })` that
   swaps the inner pair's two fields if they're in the wrong order. Which
   write needs which fact?
2. Why can a pattern bind `&mut` into the `Held` of a `&mut Slot<{ t: u8 |
   t < 100 }>`, but not into the `Some` of the `o` of a `struct P { n: u8,
   o: Option<{ v: u8 | v < n }> }`?
3. Lemma V2 checks each value as it will be, before the write. What would
   go wrong checking it after, reading the value again?

## Reading

- Nico Lehmann, Adam Geller, Niki Vazou, Ranjit Jhala, *Flux: Liquid
  Types for Rust* (PLDI 2023): a `&mut`'s pointee has a refined type that
  every write through it keeps, and a value's refinement changes with a
  write only where the value is owned.
- John C. Reynolds, *Separation Logic: A Logic for Shared Mutable Data
  Structures* (LICS 2002): the frame rule, what a write to one part
  leaves alone.
