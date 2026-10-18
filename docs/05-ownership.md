# Step 5: ownership, borrowing and pointers

Code: `src/check.rs` (references, boxes, assignment and `while` in the type
checker), `src/borrowck.rs` and `src/borrowck/` (the borrow checker), and
`src/interp.rs` (memory).

```rust
#[derive(Discard)]
enum List { Nil, Cons(int, Box<List>) }

fn sum(l: &List) -> int {
    match l {
        List::Nil => 0,
        List::Cons(x, rest) => *x + sum(rest),
    }
}

fn bump(l: &mut List) {
    match l {
        List::Nil => {},
        List::Cons(x, rest) => {
            *x += 1;
            bump(rest);
        },
    }
}

fn main() {
    let mut l = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));
    bump(&mut l);
    print(sum(&l)); // expect: 5
}
```

A list that holds itself, on the heap, changed in place through a
reference, and freed when `l` goes away, with no garbage collector and
nothing checked at run time. This step is about how a type system can
promise that's safe: that nothing is freed twice, or used after it's
freed, or changed while something else is reading it.

## The problem

Until now every value in T was immutable, lived in the frame of the
function that made it, and was copied wherever it went. That's simple and
safe, but it's not how an operating system uses memory. It needs:

- **The heap**, for values whose size or lifetime isn't known when the
  function that makes them is compiled. A list of any length can't live in
  one stack frame.
- **Pointers**, to refer to a value without copying it, and so that a type
  can hold itself (`List` holds a pointer to a `List`) with a finite size.
- **Mutation**, to change a value in place rather than build a new one.

Each of those is how memory goes wrong in C. Something must free what's on
the heap; free it twice, or use it after, and memory is corrupted. A pointer
can outlive what it points to (*dangling*). And mutating a value while
something else holds a pointer into it can invalidate that pointer: shrink
a list while someone's halfway along it.

Most languages solve all three with a garbage collector: nothing is freed
while anything can reach it. T can't have one (see the README). The other
way is to make the type system decide when memory is freed, and to rule out
the programs that would get it wrong. That's *ownership*, and it's Rust's
answer.

## Ownership and moves

Every value has exactly one *owner*: a variable, a field, a box. When the
owner goes away, so does the value, and anything it owns. A `Box<T>` owns
a `T` on the heap, so freeing the box frees the `T`:

```rust
fn main() {
    let b = Box::new(5); // allocates
    print(*b);           // expect: 5
}                        // `b` goes away: the box is freed
```

`Box::new` is the only way a T program allocates, and it's explicit:
nothing allocates behind your back. (It belongs in an `alloc` library, as
in Rust; for now it's built in.)

If owners were copied, there'd be two, and the box would be freed twice. So
using a value that isn't `Copy` *moves* it: the new place owns it, and the
old one can't be used again until it gets a new value.

```rust,error
fn main() {
    let b = Box::new(1);
    let c = b; // moved: `c` owns the box now
    print(*c);
    print(*b); // error: use of moved value `b`
}
```

A type system where a value can be used *at most once* is *affine*. It's
one of the *substructural* type systems, so called because they drop some
of the structural rules of ordinary logic, which say an assumption can be
used any number of times. Drop *contraction* (using an assumption twice)
and you get affine types; drop *weakening* (not using one at all) too, and
you get *linear* types, where every value must be used exactly once. T's
types are affine, since a value can always be dropped.

### Copy

Some types are fine to duplicate: an `int` owns nothing. Those are `Copy`:
`int`, `bool`, `()`, functions, shared references (below), and structs and
enums that say so with `#[derive(Copy)]`, if all their fields are `Copy`:

```rust
#[derive(Discard, Copy)]
struct Point { x: int, y: int }

fn main() {
    let p = Point { x: 1, y: 2 };
    let q = p;        // copied
    print(p.x + q.y); // expect: 3
}
```

Why not make every struct with `Copy` fields `Copy` without asking? Rust
did, before 1.0, and dropped it (RFC 19): then adding a `Box` field to a
struct silently stops it being copyable, which breaks code far away. Saying
`#[derive(Copy)]` makes it a promise. In step 6 `Copy` becomes a trait, and
`#[derive(Copy)]` an implementation of it; for a generic type, it's `Copy`
when its type arguments are, as `#[derive(Copy)]` means in Rust.

A type parameter `T` might stand for a `Box`, so it's never `Copy`, and
using a `T` moves it (`err_move_generic.t`).

### What a move is

A move isn't a different operation from a copy, in memory: both copy the
bits. What's different is that after a move, the old place is treated as
empty. That's why moves are checked, not done: the checker works out, at
each point in a function, which places *may* have been moved out of, and
rejects any use of one. "May": if one branch of an `if` moves `b` and the
other doesn't, `b` can't be used after the `if` (`err_move_branch.t`).

Moving out of part of a value is fine (`let a = o.a;`), and so is giving
it a new value later (`o.a = Box::new(2);`), but using the whole while a
part is missing isn't (`err_move_partial.t`). A `match` moves out only
what its arm binds (`err_move_match.t`).

## Mutation

Variables are immutable unless declared `mut`. Then they can be assigned to,
as can their fields, and `+=` and the like work on `int`s. With mutation
comes the `while` loop, which needs it to ever end:

```rust
fn fib(n: int) -> int {
    let mut a = 0;
    let mut b = 1;
    let mut i = 0;
    while i < n {
        let t = a + b;
        a = b;
        b = t;
        i += 1;
    }
    a
}

fn main() {
    print(fib(10)); // expect: 55
}
```

Assigning drops what was there: `b = Box::new(2)` frees `b`'s old box. A
variable declared `mut` that's never changed gets a warning, whose fix
removes the `mut` (`unused_mut`).

### Declaring a variable without a value

`let r;` declares a variable with no value: an assignment gives it one
later. Its type is a new type variable, which that assignment solves
(`let r: int;` says it up front). It's as if it had been moved out of, so
the move check already rejects using it before then
(`err_let_deferred_unset.t`), and an assignment gives it a value as it
would a moved-out variable. Only a whole variable can be declared this way:
an assignment can't give `x` and `y` of `let P { x, y };` their values.

```rust
fn main() {
    let x = 1;
    let y = 2;
    let r;
    if x < y {
        r = &x;
    } else {
        r = &y;
    }
    print(*r); // expect: 1
}
```

`r` isn't `mut`, but assigning to it is fine, since the assignment is how
it gets its value. That needs one more thing, though: that `r` *can't* have
a value already. If it could, the assignment would be changing it, which
needs `mut` (`err_let_deferred_twice.t`, and `err_let_deferred_loop.t` for
the second time round a loop). That's the opposite of the move check, which
asks whether a place *may* have no value, so it's another forward dataflow
problem: which variables *may* have a value (`moves::deferred`). Where it
may not, the assignment doesn't need `mut`, so `let mut r; r = 1;` gets
the `unused_mut` warning.

## Borrowing

Moving a value into a function and back out again to let it look at it
would be tedious. A *reference* lets it look without owning:

- `&x`, of type `&T`, is a *shared* reference: through it, `x` can be read,
  but not changed. There can be any number at once, so `&T` is `Copy`.
- `&mut x`, of type `&mut T`, is a *mutable* reference: through it, `x` can
  be changed. There can only be one at a time, and while it's around, `x`
  can't be used any other way.

That's the core rule: **aliasing xor mutation**. A value can be shared, or
mutated, but not both at once. It's what makes "changed while something
else is reading it" impossible:

```rust,error
fn main() {
    let mut x = 1;
    let r = &x;
    x = 2; // error: cannot assign to `x` while `x` is borrowed
    print(*r);
}
```

`*r` is what `r` points to, and a field of a reference is a field of what
it points to: `p.x` for `p: &Point` is `(*p).x`. A `&mut T` can be used
where a `&T` is wanted, and a `&Box<T>` or `&&T` where a `&T` is, by
following the pointers (*deref coercion*): that's how `sum(rest)` above
passes a `&Box<List>` as a `&List`. The checker writes these out, as
`&**rest`, before anything after it sees the program.

A borrow mustn't outlive what it borrows, either. Here `x` is freed at the
`}`, while `r` still points to it:

```rust,error
fn main() {
    let mut r = &0;
    {
        let x = 1;
        r = &x;
    } // error: `x` doesn't live long enough
    print(*r);
}
```

### Regions

How long is a borrow? The checker gives each reference type a *region*,
written `'a`: the set of points in the program where it may be used.
`&'a T` is a reference that's good for region `'a`. The rules are that a
reference's region must be inside the region where what it borrows is
alive, and no use of what it borrows may clash with the borrow anywhere in
the region.

Regions go back to Tofte and Talpin's *region inference* (1997), where
they decided where in memory values went, and to Cyclone (2002), a safe
C that checked pointers with them. Rust calls them *lifetimes*, and so does
T's syntax. A function says how the lifetimes of what it takes and returns
are related:

```rust
fn longer<'a>(x: &'a int, y: &'a int) -> &'a int {
    if *x >= *y { x } else { y }
}

fn main() {
    let a = 1;
    let b = 2;
    print(*longer(&a, &b)); // expect: 2
}
```

`'a` is a *lifetime parameter*, generic like a type parameter: each call
picks a region for it, and the result can only be used where both
arguments are alive. Most signatures needn't write them, thanks to
*lifetime elision*, as in Rust: each reference parameter gets a lifetime of
its own, and if there's exactly one, the result has it. `fn first(p:
&Pair) -> &int` means `fn first<'a>(p: &'a Pair) -> &'a int`. With two,
it has to be written (`err_lifetime_elision.t`).

A struct can hold a reference, given a lifetime parameter, and that's the
other way a type can hold itself, with nothing on the heap
(`tests/t/lifetimes.t`):

```rust
#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

#[derive(Discard)]
struct Node<'a> { value: int, next: Option<&'a Node<'a>> }

fn total(n: &Node) -> int {
    match n.next {
        Option::Some(next) => n.value + total(next),
        Option::None => n.value,
    }
}

fn main() {
    let c = Node { value: 3, next: Option::None };
    let b = Node { value: 2, next: Option::Some(&c) };
    let a = Node { value: 1, next: Option::Some(&b) };
    print(total(&a)); // expect: 6
}
```

## The borrow checker

Type checking (`check.rs`) treats regions as if they weren't there: `&'a
int` and `&'b int` are the same type to it. Working them out is a separate
pass, the *borrow checker* (`borrowck.rs`), which runs on each function
once its types are known. T's is modelled on rustc's since 2018:
*non-lexical lifetimes* (NLL, RFC 2094). Non-lexical, because a borrow
lasts as long as the reference is used, not to the end of the block it's
in:

```rust
fn main() {
    let mut x = 1;
    let r = &x;
    print(*r); // expect: 1
    x = 2;     // fine: `r` isn't used after this
    print(x);  // expect: 2
}
```

### MIR

The checker doesn't work on the AST but on a lower-level form of each body,
*MIR* (`borrowck/mir.rs`, `borrowck/build.rs`): a graph of basic blocks of
simple statements, where every value is in a numbered local, every
statement reads and writes *places* (a local followed by fields and `*`s,
like `(*_3).1`), and every branch, loop, temporary and end of scope is
explicit. A program point is then just a place in that graph, which is what
regions are sets of. Roughly:

```text
let r = &x;      _3 = &'?7 _1        // a loan of _1, for region '?7
print(*r);       _2 = copy _3
                 _4 = copy *_2
                 _5 = call print(_4)
x = 2;           _1 = 2              // a write to _1
```

### Working out regions

Each region in each local's type gets a variable, and two kinds of
constraint say what's in them (`borrowck/regions.rs`):

- **Liveness.** If a local may still be used at a point, every region in
  its type contains the point. A reference must be valid wherever it might
  be used. Which locals are *live* where is a classic backwards dataflow
  analysis.
- **Outlives**, `'a: 'b`, read "`'a` outlives `'b`": `'a` contains
  everything `'b` does. Wherever a value of type `&'a T` goes into a place
  of type `&'b T`, `'a: 'b`, since the place's region is where the value
  will be used. That's *subtyping*: `&'a T` is a subtype of `&'b T` when
  `'a: 'b`, a longer-lived reference usable as a shorter-lived one.

Then the smallest regions that satisfy the constraints are found by growing
them until nothing changes. In the program above, `r`'s region is the two
points between `let r` and `print(*r)`, since `r` is dead after that.

Each `&` makes a *loan*, of a place, for a region. Then a forward dataflow
analysis finds which loans are *in scope* at each point, those whose region
contains it and that weren't cut short by their place getting a new value,
and every access is checked against them (`borrowck/loans.rs`): reading
clashes with a `&mut` loan of an overlapping place, and writing, moving,
`&mut` and going out of scope clash with any. `x = 2` is after `r`'s
region, so `x`'s loan isn't in scope there, and nothing clashes.

The same machinery finds use-after-move (`borrowck/moves.rs`): which places
*may* have been moved out of flows forward the same way, and an
assignment to a variable that isn't `mut` is an error, with a fix.

### Variance

Subtyping on references raises a question: if `&'long int` is a `&'short
int`, is a `&mut &'long int` a `&mut &'short int`? It mustn't be:

```rust,error
fn assign<'a>(r: &mut &'a int, v: &'a int) {
    *r = v;
}

fn main() {
    let x = 1;
    let mut r = &x;
    {
        let y = 2;
        assign(&mut r, &y);
    } // error: `y` doesn't live long enough
    print(*r);
}
```

If it were, `assign` could write the short-lived `&y` into `r`, which
outlives `y`. So `&'a mut T` is *invariant* in `T`: a `&mut T` is only ever
a `&mut T` for the very same `T`, regions and all. `&'a T` and `Box<T>`
are *covariant* (a subtype inside makes a subtype outside), and function
types are *contravariant* in their parameters: a function that takes any
`&'short int` can be used as one that takes `&'long int`s, not the other
way round. A struct or enum's variance in each parameter comes from how its
fields use it, worked out to a fixpoint (`AdtInfo::new`).

### What a signature promises

Inside a function, its lifetime parameters are *universal* regions: they
contain the whole body, and the end of themselves, somewhere in the caller.
Nothing more is known about them than the signature says. Returning `y` as
a `&'a int` needs `'b: 'a`, which it doesn't say:

```rust,error
fn second<'a, 'b>(_x: &'a int, y: &'b int) -> &'a int { y }
// error: lifetime may not live long enough: this needs `'b` to outlive `'a`
```

Bounds say what the signature promises: `<'a, 'b: 'a>`. So do the types
themselves: a `&'a &'b int` only makes sense if `'b: 'a`, so a function
taking one may assume it (*implied bounds*). Callers have to show it, at
each call. That last part is where Rust's longest-standing soundness bug
lives (rust#25860, the root of `cve-rs`): implied bounds of a function
*pointer's* type are assumed, but nothing checks them when a function is
turned into one. T checks them whenever a function is used, called or not
(`Builder::fn_value`), and assumes none from function types.

A type parameter is a universal type, and `T: 'a` says every reference in a
`T` outlives `'a`. A `&'a T` implies it; anything else needs the bound
(`err_lifetime_param_outlives.t`).

### Function types

What's `fn(&int) -> int`? Not a function that takes references of one
particular lifetime, which would be useless: nothing could pass it a
reference to a local. It's one that takes a reference of *any* lifetime,
like a generic function: `for<'x> fn(&'x int) -> int`, in Rust's syntax.
A lifetime left out in a function type is bound by it (`FnType::bound`),
and each call picks a new one. That makes function types *higher-ranked*
in their lifetimes, a small corner of System F (docs/04-generics.md) that
monomorphisation doesn't mind, since lifetimes don't exist at run time.

```rust
fn apply(f: fn(&int) -> int, x: &int) -> int { f(x) }

fn main() {
    let n = 3;
    print(apply(|r| *r + 1, &n)); // expect: 4
}
```

Using a function where a higher-ranked type is wanted means checking it
works for any lifetime: each is a new *placeholder* region, universal like
a lifetime parameter, which the function mustn't need anything of. A
function that only takes `&'static int`s isn't *general enough*
(`err_lifetime_fn_type.t`).

### Closures

A closure's body is checked as a function of its own, with the regions in
its signature as its universal regions. What it needs of those, it can't
decide, so it's passed up to the function it's in, as constraints on the
closure's type there: that's how `|r| r` checks against `fn(&int) ->
&int`.

What closures capture is unchanged from step 3: copies, and closures that
capture are downward-only. With references and boxes about, a closure can
only capture what's `Copy` and has no lifetime in its type, so a copy can't
dangle and can't be freed twice (`err_closure_capture_box.t`,
`err_closure_capture_ref.t`). Its copies are its own, and it can't change
them (`err_mut_closure_capture.t`). Giving function types lifetimes, so a
closure can capture references and be returned with them, waits for traits
(step 6), when each closure gets a type of its own.

## Matching through references

Matching on a reference looks through it, and binds variables *by
reference*, to what they match inside the value it points to. That's
Rust's *match ergonomics* (RFC 2005): in `match l { List::Cons(x, rest) =>
.. }` with `l: &List`, `x` is a `&int` and `rest` a `&Box<List>`, and
nothing is moved out of `*l`, which would be an error, since it's behind a
reference (`err_move_behind_ref.t`). Through a `&mut`, they're `&mut`s,
which is how `bump` changes the list in place.

### Types with no values

A `match` needn't cover what can't exist (docs/02-adts.md), and pointers
add to that. A reference always points to a value, and a box always holds
one, so a `&Never` or `Box<Never>` can't exist either. Neither can a value
of `struct Stream { next: Box<Stream> }`: values are finite, so a chain of
boxes has to end somewhere, and a `Stream`'s can't
(`match_uninhabited_pointer.t`). In a lazy language like Haskell, where a
value can be infinite, that's a perfectly good stream. In T it's almost
certainly a list missing its `Nil`, so it gets a warning
(`infinite_types`), as do types that hold each other that way
(`warn_infinite_types.t`).

Rust asks for the arms anyway, for references: unsafe code there may make
a `&Never` out of raw memory, and it hasn't settled whether that's allowed.
T settles it: a reference to something that isn't a valid value is wrong,
in unsafe code too, once there is any.

## The interpreter keeps memory honestly

The interpreter's job is to give programs a meaning to check the type
system against, and until now it could afford to copy every value. Now it
keeps memory as a compiled program would (`interp.rs`): every variable and
temporary in a *slot*, freed when it goes out of scope; a box's value in a
slot on the heap, freed when the box is dropped; a reference as a slot and a
path to a field in it. Moving out of a place leaves `Value::Moved` behind.
Then a dangling reference, a use after a move, a double free and a leak (a
box that's never freed) are all caught as it runs, with the same
`unsound(..)` as a type error that slipped through. The borrow checker
promises none of them happen, and every test checks that promise.

That only works if the interpreter and the borrow checker agree on when
memory goes away, so the rules are written down once and both follow
them: a statement's temporaries go at its end, a block's variables at its
`}`, the last declared first, and `let r = &f();` keeps `f()`'s temporary
for as long as `r` (`err_borrow_temp.t`).

## Design decisions worth noticing

- **Moves by default, `#[derive(Copy)]` to copy.** See *Copy*.
- **Lifetimes as in Rust**, explicit where elision can't tell, and checked
  NLL-style, on MIR. A lexical checker (borrows last to the end of their
  block) is simpler but rejects too much; Polonius, Rust's next one, accepts
  more than NLL, at the cost of computing with loans rather than points.
- **Reborrowing.** Passing a `&mut` somewhere borrows what it points to
  again (`&mut *r`), so it can be used again after: `deposit(m, 5);
  deposit(m, 5);`. Rust does this where it knows a `&mut` is wanted; T does
  it wherever a `&mut` place is used as a value, which accepts a little more
  and is no less sound.
- **No `Drop`.** Dropping only ever frees boxes, so it never uses the
  references in what it drops, and a struct can hold references to things
  that go away before it does, as long as it isn't used after.
- **Deferred initialisation for whole variables only.** `let r;` reuses
  the move check, which already knows what has no value. A pattern would
  need each of its variables assigned on its own (exercise 5).
- **`Box` is built in, and allocation can't fail.** In step 7, allocating is
  an effect, and running out of memory a panic that a function's type says
  it may have.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `Rc`, `RefCell`, sharing that's checked at run time | a library, once there's `alloc`; `RefCell` panics | — |
| raw pointers, `unsafe` | for writing what the checker can't see is safe, like `Vec` itself | — |
| a resizable array (`Vec`) | needs raw pointers, generics over sizes, and `alloc` | — |
| closures capturing references, returning closures | function types with lifetimes, a type per closure | 6 |
| `Drop`, destructors | traits | 6 |
| allocation that may fail | effects | 7 |
| `v.push(v.len())`: a call's borrows taken after its arguments | no methods yet | 6 |

`break`, `continue`, `return` and `loop`, control flow the MIR already had
room for, came right after: docs/05.1-control-flow.md.

## Exercises

1. Write a program that the borrow checker rejects but that would run
   fine. (There are plenty: the checker can't see which branch an `if`
   takes.) Then one it rejects that would go wrong if it ran: exercise 2
   shows how to see what would happen.
2. Turn off the move check (`moves::moves`) and write a program that moves
   a box twice. The interpreter reports a use after move, not a double
   free: why? Then turn off the loans check and make it report a dangling
   reference.
3. Give `&mut T` the variance of `&T`, covariant in `T`, and make the
   program in *Variance* run. What does the interpreter say?
4. Walk the borrow checker through `nll.t`'s loop by hand: which region
   holds which points, and why is each loan out of scope by the next time
   round?
5. `let r;` works for a variable only. Make it work for any irrefutable
   pattern, `let P { x, y };`, each variable given its value by an
   assignment of its own. What does `moves::deferred` need to change?
6. `fn f<'a>(x: &'a mut int) -> &'a int { x }` takes a mutable reference
   and returns a shared one. Is `x`'s referent still mutably borrowed while
   the result is in use? Should it be?
7. `Cx::inhabited_in` in `exhaust.rs` says `struct Stream { next:
   Box<Stream> }` has no values, by taking a type it's already building a
   value of to have none. Why is that right? Would taking it to have some
   be wrong, or just less useful?
8. Read rust#25860. Write the T version of the program in it, and say
   which check stops it.

## Reading

- David Walker, *Substructural Type Systems*, chapter 1 of *Advanced Topics
  in Types and Programming Languages* (Pierce, ed.), 2005.
- Mads Tofte and Jean-Pierre Talpin, *Region-Based Memory Management*,
  Information and Computation, 1997.
- Dan Grossman et al., *Region-Based Memory Management in Cyclone*, PLDI
  2002.
- Niko Matsakis, RFC 2094: *Non-lexical lifetimes*, 2017. The design of
  rustc's borrow checker, and most of this one.
- Ralf Jung et al., *RustBelt: Securing the Foundations of the Rust
  Programming Language*, POPL 2018. A proof that the rules are sound.
- Aaron Weiss et al., *Oxide: The Essence of Rust*, 2019. A formal model of
  Rust's borrow checker, with regions as sets of loans, as Polonius has them.
- rust-lang/rust issue #25860, *Implied bounds on nested references +
  variance = soundness hole*, and the `cve-rs` crate built on it.
