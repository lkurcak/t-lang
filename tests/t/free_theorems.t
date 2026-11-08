// A type parameter only marker traits bound can't be made from nothing, so
// what a call gives of it is one of what it was given: refinements go
// through `unwrap`, `unwrap_or`, `?` and generic functions of the
// program's own (docs/55-free-theorems.md).

use core::ops::ControlFlow;
use core::ops::Try;

fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn sorted(n: usize) -> Result<{ v: usize | v < 4 }, { e: usize | e >= 4 }> {
    if n < 4 { Ok(n) } else { Err(n) }
}

// `unwrap` gives what's in the `Some`; `expect` too, of a `Result`.
fn at(a: [u8; 4], n: usize) -> u8 with panic { a[small(n).unwrap()] }

fn at_ok(a: [u8; 4], n: usize) -> u8 with panic { a[sorted(n).expect("small")] }

// By path, too.
fn at_path(a: [u8; 4], n: usize) -> u8 with panic { a[Option::unwrap(small(n))] }

// `unwrap_or` gives what's in the `Some`, or the default.
fn at_or(a: [u8; 4], n: usize) -> u8 { a[small(n).unwrap_or(3)] }

// Once `unwrap` has returned, the value was an `Ok`: its `Err` arm can't be
// taken, though what an `Err` holds is only known to be 4 or more.
fn was_ok(a: [u8; 4], n: usize) -> u8 with panic {
    let r = sorted(n);
    let _ = r.unwrap();
    match r {
        Ok(i) => a[i],
        Err(e) => a[e - 4],
    }
}

// `?` gives what's in the `Some`.
fn at_try(a: [u8; 4], n: usize) -> Option<u8> {
    let i = small(n)?;
    Some(a[i])
}

// A function whose result is refined can use `?`: what goes up is a `None`.
fn double(n: usize) -> Option<{ v: usize | v < 7 }> {
    let i = small(n)?;
    Some(i + i)
}

// An error's refinement goes up through `?`.
fn up(n: usize) -> Result<usize, { e: usize | e >= 4 }> {
    let i = sorted(n)?;
    Ok(i)
}

fn past(n: usize) -> usize {
    match up(n) {
        Ok(_) => 0,
        Err(e) => e - 4,
    }
}

// `ok` keeps what's in the `Ok`, and its `Some` says it was an `Ok`;
// `err` the same of the `Err`.
fn at_ok_option(a: [u8; 4], n: usize) -> u8 {
    match sorted(n).ok() {
        Some(i) => a[i],
        None => 0,
    }
}

fn err_past(n: usize) -> usize {
    match sorted(n).err() {
        Some(e) => e - 4,
        None => 0,
    }
}

// `ok_or` makes the `Some` an `Ok`, and the error is the one given.
fn at_ok_or(a: [u8; 4], n: usize) -> Result<u8, u8> {
    let i = small(n).ok_or(9u8)?;
    Ok(a[i])
}

fn missing(n: usize) -> u8 {
    match small(n).ok_or(9u8) {
        Ok(_) => 0,
        Err(e) => e - 9,
    }
}

// `into_ok` of a `Result` that can't be an `Err`.
fn sure(n: { v: usize | v < 4 }) -> Result<{ v: usize | v < 4 }, !> { Ok(n) }

fn at_sure(a: [u8; 4], n: { v: usize | v < 4 }) -> u8 { a[sure(n).into_ok()] }

// `branch` by itself, which `?` is made of.
fn at_branch(a: [u8; 4], n: usize) -> u8 {
    match Try::branch(small(n)) {
        ControlFlow::Continue(i) => a[i],
        ControlFlow::Break(_) => 0,
    }
}

// A generic function of the program's own: one of its arguments.
fn pick<T: Copy>(first: bool, a: T, b: T) -> T {
    if first { a } else { b }
}

fn at_pick(a: [u8; 4], c: bool, i: { v: usize | v < 4 }) -> u8 { a[pick(c, i, 2)] }

// A refinement of a value that isn't an integer: the reference given back
// is the one given, so it has the same length.
fn first_of(s: Option<{ s: &[u8] | s.len() == 4 }>) -> u8 with panic {
    let s = s.unwrap();
    s[3]
}

// An enum of the program's own, and a type parameter of a trait's method,
// whose impls can't need more of it.
#[derive(Discard)]
enum Either<L, R> { Left(L), Right(R) }

fn left_or<L: Copy, R: Discard>(e: Either<L, R>, d: L) -> L {
    match e {
        Either::Left(l) => l,
        Either::Right(_) => d,
    }
}

fn at_left(a: [u8; 4], left: bool, i: { v: usize | v < 4 }) -> u8 {
    let e: Either<usize, bool> = if left { Either::Left(i) } else { Either::Right(left) };
    a[left_or(e, 1)]
}

trait Chooser {
    fn choose<T: Copy>(&self, a: T, b: T) -> T;
}

#[derive(Copy, Discard)]
struct Second {}

impl Chooser for Second {
    fn choose<T: Copy>(&self, _a: T, b: T) -> T { b }
}

fn by<C: Chooser>(c: &C, a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[c.choose(i, 3)] }

// A function that gives a `T` it was never given doesn't return.
fn forever<T>() -> T {
    loop {}
}

fn unreached(a: [u8; 4], go: bool) -> u8 {
    if go {
        let i: usize = forever();
        a[i + 9]
    } else {
        0
    }
}

// The arithmetic that can't overflow says what it gives.
fn sum(t: [u8; 7], i: { v: usize | v < 4 }, j: { v: usize | v < 4 }) -> u8 {
    match checked_add(i, j) {
        Some(k) => t[k],
        None => 0,
    }
}

fn product(t: [u8; 10], i: { v: usize | v < 4 }, j: { v: usize | v < 4 }) -> Option<u8> {
    let k = checked_mul(i, j)?;
    Some(t[k])
}

fn difference(t: [u8; 4], i: { v: usize | v < 4 }, j: usize) -> Option<u8> {
    Some(t[checked_sub(i, j)?])
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [5, 6, 7, 8];
    out.line(&at(a, 1))?;                     // expect: 6
    out.line(&at_ok(a, 2))?;                  // expect: 7
    out.line(&at_path(a, 3))?;                // expect: 8
    out.line(&at_or(a, 0))?;                  // expect: 5
    out.line(&at_or(a, 9))?;                  // expect: 8
    out.line(&was_ok(a, 2))?;                 // expect: 7
    out.line(&at_try(a, 1).unwrap_or(0))?;    // expect: 6
    out.line(&at_try(a, 7).unwrap_or(0))?;    // expect: 0
    out.line(&double(3).unwrap_or(0))?;       // expect: 6
    out.line(&double(4).unwrap_or(0))?;       // expect: 0
    out.line(&past(6))?;                      // expect: 2
    out.line(&at_ok_option(a, 3))?;           // expect: 8
    out.line(&err_past(5))?;                  // expect: 1
    out.line(&at_ok_or(a, 0).unwrap_or(0))?;  // expect: 5
    out.line(&missing(4))?;                   // expect: 0
    out.line(&at_sure(a, 2))?;                // expect: 7
    out.line(&at_branch(a, 1))?;              // expect: 6
    out.line(&at_branch(a, 4))?;              // expect: 0
    out.line(&at_pick(a, true, 0))?;          // expect: 5
    out.line(&at_pick(a, false, 0))?;         // expect: 7
    out.line(&first_of(Some(a.as_slice())))?; // expect: 8
    out.line(&at_left(a, true, 2))?;          // expect: 7
    out.line(&at_left(a, false, 2))?;         // expect: 6
    out.line(&by(&Second {}, a, 0))?;         // expect: 8
    out.line(&unreached(a, false))?;          // expect: 0
    let t: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let s: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
    out.line(&sum(s, 3, 3))?;                     // expect: 6
    out.line(&product(t, 3, 3).unwrap_or(0))?;    // expect: 9
    out.line(&difference(a, 3, 1).unwrap_or(0))?; // expect: 7
    out.line(&difference(a, 1, 3).unwrap_or(0))?; // expect: 0
    Ok(())
}
