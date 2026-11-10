# Step 62: what t-chess found

Code: `valid` and `connected` in `src/check/refine/logic.rs`;
`pattern_formula`, `pattern_implies`, `field_patterns`, `bind_pattern`,
`bound_mut`, `said_of` and `Walk::failed` in `src/check/refine.rs`;
`Error::more` in `src/error.rs`; T-Cmp in `src/check/expr.rs`;
`Printer::use_tree` in `src/pretty.rs`; `DoubleEndedIterator`, `rev`,
`any` and `all` in `core/iter.t`; `slice::Copied` in `core/slice.t`;
`to_ascii_uppercase` and `to_ascii_lowercase` in `core/char.t`.

A terminal chess game, written years ago with `-1` for "no square", was
ported to 0.77.0 with every square a `{ s: u8 | s < 64 }` and `main` saying
`with alloc`, so that nothing in it can panic. Its friction log had eight
findings. This step fixes six: a proof that the solver took too long to
find, refinements a pattern forgot, comparisons of characters that said
nothing, `use` lists `t fmt` wouldn't break, a few missing iterator and
character methods, and a stale sentence in the reference. Looking into the
second turned up a hole that had been there since step 60.

## Facts the goal doesn't need

The game's `apply` takes away castling rights with three statements like
`if from == 7 || to == 7 { g.a = false; }`, handles castling in a branch
that lends `g` to a call, and then works out the square a pawn skipped:

```rust
#[derive(Discard)]
struct Game { a: bool, b: bool, c: bool, n: u8, ep: Option<u8> }

fn put(g: &mut Game, s: { s: u8 | s < 64 }) { g.n = s }

fn apply(g: &mut Game, from: { s: u8 | s < 64 }, to: { s: u8 | s < 64 }, pawn: bool) with {} {
    if from == 7 || to == 7 {
        g.a = false;
    }
    if from == 56 || to == 56 {
        g.b = false;
    }
    if from == 63 || to == 63 {
        g.c = false;
    }
    if from == 4 || from == 60 {
        if to == from + 2 {
            put(g, from + 3);
        } else if to + 2 == from {
            g.n = from - 4;
        }
    }
    g.ep = if pawn && (to == from + 16 || from == to + 16) { Some((from + to) / 2) } else { None };
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut g = Game { a: true, b: true, c: true, n: 0, ep: None };
    apply(&mut g, 12, 28, true);
    out.line(&g.ep.unwrap_or(0))?; // expect: 20
    Ok(())
}
```

`to + 16` fits a `u8` because `to <= 63`, which is known; with 0.77.0 the
proof took seconds and then gave up after 512 cases. Each `if` joins
two paths into a disjunction, which the solver splits into cases, and step
61's backjumping should have made the ones the proof doesn't need cost one
case each. It didn't, for two reasons. The goal's own disjunction (`to <
-16 || to > 239`, the negation of what's to be proved) was split last, 27
splits deep. And when a case was refuted, the splits it was said to depend
on included ones it didn't need: the solver substitutes an equation such as
`from == 60` into every constraint mentioning `from`, and each of those
then depends on the split the equation came from. So backjumping went back
to splits it could have skipped.

Now the goal's negation is split first, and then what's known, in order of
how far it is from the goal: first the facts about the goal's variables,
then those about their variables, and so on (`connected`, by layers of a
breadth-first search over shared variables). The facts without
disjunctions are in every case, so a goal they prove, as here, is proved
by the first two cases, before any of the facts' disjunctions is split.
The game's module checks in under a second, as fast as with the
workaround it used, and `tests/proof_work.rs` checks that the work stays
the same with any number of such `if`s.

## Patterns of several parts

The move parser wants both squares at once:

```rust
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> {
    if n < 64 { Some(n) } else { None }
}

fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn both(a: u8, b: u8) -> u8 {
    let (Some(x), Some(y)) = (square(a), square(b)) else { return 0 };
    need(x) + need(y)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&both(3, 4))?;  // expect: 7
    out.line(&both(3, 99))?; // expect: 0
    Ok(())
}
```

`square`'s result says its `Some` is below 64: a fact that holds *if* the
option is `Some`. A `match` on an option says which variant an arm has, but
a tuple pattern said nothing of its parts, so neither `x` nor `y` knew
anything. (The friction log thought one `Some` in a tuple worked; none
did.) Now a tuple pattern matches exactly when each part matches its
pattern, and so does a struct pattern, field by field, by name: the arm
knows which variants the parts are, and so what their types say of their
fields. Through a reference too: `match &t { (Some(a), _) => .. }`.

## A hole: writing through a `&mut` match made

Making tuple patterns say more exposed a hole that had been there since
step 60. This was accepted, and now isn't:

```rust,error
fn poke(o: &mut Option<{ s: u8 | s < 64 }>) {
    match o {
        Some(a) => *a = 99,
        None => {},
    }
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
```

Match ergonomics binds `a` as a `&mut u8` into the option's field. Since
step 60, what a write must keep is carried by places and objects, not by
types; but a variant's field isn't a place the checker follows, so `a`
carried nothing, and `*a = 99` was checked against nothing. The option
`poke` was lent then held 99 where its type says something below 64, and
the interpreter stopped on it. The same happened through a field of a
refined `let` (`match &mut t.0`), through a `&mut` a call gave back, and
for what an or-pattern's alternative binds (`E::A(a) | E::B(a)`), an
array pattern's element (`[a, _]`) and a tuple's field when the tuple
isn't at a place (`match get(&mut t) { (a, _) => .. }`): none of them has
a place the checker follows.

A pattern now can't bind any of those by `&mut` where anything is said of
the value it's in: what's declared of the place, a refined local, a
struct field a refinement is about, or what's carried of the value. That
is the rule fields of structs already had. Where nothing is said, as in an
`Option<u8>`, it binds as before, and what was known of the value is
forgotten after the write. A binding by `&mut` of something that has a
place, a tuple's or a struct's field, is now bound as `let r = &mut p`
binds `r`, so what's written through it is checked against what's declared
of `p`.

## Every failed proof

The port also found that only the first failed proof was reported, which
made it look as if a fix had worked when the error had only moved. Now
checking goes on past a proof that fails, as if it had been proved: what it
failed to prove is assumed after it. So no failure is a consequence of an
earlier one, and each one is worth reading:

```rust,error
fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn a(x: u8) -> u8 { need(x) }

fn b(x: u8) -> u8 {
    let y = need(x + 1);
    need(y) + need(x)
}
// error: can't prove `s < 64` for argument `s` of `need`
// error: can't prove `s < 64` for argument `s` of `need`
// error: can't prove `s < 64` for argument `s` of `need`
```

`need(x)` in `b` isn't reported: the failure of `need(x + 1)` assumed
`x + 1 < 64`, which proves `x < 64`. `need(y)` is, as `need` says nothing
of what it gives. Any other error still stops checking, and after
a failed proof the passes over the whole program (unused items, effects a
function says but doesn't have) aren't run. `t check` prints each error,
and the language server shows each.

## Characters compared

```rust
fn square(file: char, rank: char) -> Option<{ s: u8 | s < 64 }> with {} {
    if file < 'a' || file > 'h' || rank < '1' || rank > '8' {
        return None;
    }
    Some(((rank as u32 - '1' as u32) * 8 + (file as u32 - 'a' as u32)) as u8)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&square('e', '4').unwrap_or(99))?; // expect: 28
    out.line(&square('i', '4').unwrap_or(99))?; // expect: 99
    Ok(())
}
```

Since step 17.1, `<` of anything but integers is `Ord`'s `lt`, of which the
checker knows nothing. Characters were compared that way too, so the guard
said nothing about `file as u32`. Now `<`, `<=`, `>` and `>=` of two `char`s
are the machine's comparison of their numbers, as for integers, and the
checker knows they're comparisons of `c as u32`.

## `use` lists

`t fmt` wrote a `use` list on one line however long it was. Now one that
doesn't fit in 100 columns breaks as every other list does, a name to a
line, with a comma after the last:

```text
use crate::board::{
    BISHOP,
    Game,
    KING,
    step,
};
```

## Small things

- `DoubleEndedIterator`, in the prelude: an iterator that can also give
  values from the back, with `next_back`. Ranges, slices' `iter` and
  `iter_mut`, and what `map`, `rev` and `copied` make of those are.
- `Iterator::rev`: the values back to front, of a `DoubleEndedIterator`.
- `Iterator::any` and `Iterator::all`: whether a closure says `true` of
  any, or of every, value. Each stops at the first value that decides it,
  and leaves the rest to iterate.
- `copied` of a slice's iterator (`a.iter().copied()`, of an array or a
  list too): the values themselves, not references to them.
- `char::to_ascii_uppercase` and `to_ascii_lowercase`.

```rust
const KNIGHT: [(i8, i8); 3] = [(1, 2), (2, 1), (-1, 2)];

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    for k in (0..3u8).rev() {
        out.put(&k)?;
    }
    out.line("")?; // expect: 210
    for (dx, dy) in KNIGHT.iter().copied() {
        out.put(&(dx + dy))?;
    }
    out.line("")?;                                         // expect: 331
    out.line(&KNIGHT.iter().any(|d: &(i8, i8)| d.0 < 0))?; // expect: true
    out.line(&'k'.to_ascii_uppercase())?;                  // expect: K
    Ok(())
}
```

## Why it's sound

- **Lemma L1 (goal first).** The order in which disjunctions are split
  doesn't change which cases there are, only which are looked at first;
  backjumping's invariant (step 61's L2) doesn't depend on the order.
  `connected` takes the same facts as before, those connected to the goal
  by shared variables, and only orders them.
- **Lemma P1 (patterns of several parts).** A tuple value matches a tuple
  pattern exactly when each of its parts matches the pattern in its place,
  and a struct value a struct pattern when each field named matches its
  pattern (step 58's T5). The walk's value of a tuple or struct holds its
  fields' values, so the conjunction of their exact conditions is exact.
  If any part's condition isn't known, the whole is unknown, never the
  rest of it: the arms after assume the negation of what each arm before
  matched, which a partial condition would make too strong
  (`tests/t/err_soundness_tuple_pattern_partial.t`). A `&t` matched is
  `t`, as `let r = &t; match r` always was.
- **Lemma P2 (`&mut` bindings).** A write through a reference must keep
  what's declared of where it writes (step 60's A6). A variant's field, an
  array's element, an or-pattern's binding and a field of what's at no
  place have no place the walk can check a write to, so a `&mut` to one
  can only be made where nothing is declared of it: nothing is declared of the field
  unless something is said of the value it's in, which `said_of` asks of
  the place (what it declares, the refined local and the constrained
  struct fields around it) and of the value (what's carried of it). A
  `&mut` to a place declares what the place does, as `let r = &mut p`
  does, so writes through it are checked.
- **Lemma C1 (characters compared).** Core's `Ord` for `char` is
  `compare(*self as u32, *other as u32)`, and only core can implement
  `Ord` for `char`, as only core can for the integers (step 17.1's L3): so
  the machine's comparison of their numbers is what `Ord` says, generic
  code or not. A character is its number in the logic, as `c as u32` is.
- **Lemma D1 (every failed proof).** Reporting more errors can't accept a
  program: a failed proof is an error. Assuming what failed only decides
  which later failures are reported.

The adversarial tests are `tests/t/err_soundness_tuple_pattern_*.t`,
`err_soundness_*_mut_binding*.t` and `err_char_compare_bound.t`.

## What it doesn't do

- `copied` is only for slices' iterators. `Iterator::copied`, for any
  iterator of references, is an impl for `Copied<I>` where `I:
  Iterator<Item = &'a T>`, which needs `T: 'a`, and an impl of a trait
  can't say that in T. It would follow from `Item = &'a T` if every
  impl's associated types were checked to make sense where they're
  defined, which T doesn't check yet: its own item.
- Adapters still don't carry what's known of their values (step 61):
  `for r in (0..8u8).rev()` doesn't know `r < 8`. The game still counts up
  and works out `7 - k`.
- A pattern that binds a variant's field by `&mut` is refused where
  something is said of the value, even if it only reads through it:
  `match o { Some(a) => need(*a), .. }` of an `o: &mut Option<{ s: u8 | s
  < 64 }>` must be `match *o` or `match &*o`. And of a value nothing is
  declared of, what's known is forgotten after such a match.
- Two other findings stay open: a count of a list's elements can't be
  proved below its length (the game uses `saturating_add`), and a
  refinement has no name (`type Square = { s: u8 | s < 64 }`).

## Decisions

- D1, every failed proof is reported, each assumed after it fails; other
  errors stop checking.
- D2, a `use` list that doesn't fit breaks a name to a line, as every list
  does.
- D3, `rev` through `DoubleEndedIterator`, Rust's trait.
- D4, `any` and `all` take `&mut self`, as in Rust, so what's left can be
  iterated after.
- D5, `copied` of slices' iterators only, until associated types are
  checked where they're defined.

## Exercises

1. Write a goal whose proof needs a disjunctive fact two layers from it,
   with disjunctions about its own variables split first. How many cases
   does it take? What would a solver that learned from each refuted case
   take?
2. Why is `(Some(a), _) | (_, Some(a))` fine as a pattern for P1, but a
   pattern of one part whose condition isn't known not? Write the program
   that a partial condition would accept wrongly.
3. Give variants' fields places in the walk (a projection for "field `j`
   of variant `v`"), so that `match o { Some(a) => *a = 5 }` could be
   checked against `o`'s refinement instead of refused. What must a write
   to such a place do to the tag?
4. Why does assuming a failed proof never hide a failure that's
   independent of it?
5. `t fmt` breaks a list a name to a line. Find a `use` list for which
   filling lines, as rustfmt does, makes a smaller diff when a name is
   added, and one for which it makes a bigger one.

## Reading

- Larry Wos, George Robinson and Daniel Carson, *Efficiency and
  completeness of the set of support strategy in theorem proving* (1965):
  starting from the negated goal, so that what's derived is about it.
- The Rust Reference, *Binding modes* (match ergonomics), and RFC 2005,
  which introduced them.
