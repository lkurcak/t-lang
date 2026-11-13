// A generic `&mut T` keeps what's declared of the place lent itself, not
// only of what's in it: `swap`, `replace` and a `set` of our own write
// back only values of `T` they were given, each of which has the
// refinement (docs/74-refined-instantiation.md).
use alloc::list::List;
use core::mem::{replace, swap};

fn set<T: Discard>(a: &mut T, v: T) { *a = v; }

fn with_mut<T, F: FnOnce(&mut T) + Discard>(a: &mut T, f: F) { f(a) }

// A field's refinement names another field: what's lent is closed at
// what the struct's fields are now, and the struct's refinements are
// proved again after.
#[derive(Discard)]
struct Band {
    rows: usize,
    now: { l: List<u8> | l.len() == rows },
    next: { l: List<u8> | l.len() == rows },
}

fn flip(b: &mut Band) {
    swap(&mut b.now, &mut b.next);
}

// A field's refinement about it alone.
#[derive(Copy, Discard)]
struct Pos {
    n: { v: i64 | v > 0 },
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: { v: u32 | v > 0 } = 1;
    let mut b: { v: u32 | v > 0 } = 2;
    swap(&mut a, &mut b);
    let old = replace(&mut a, 7);
    set(&mut b, old);
    let c: { v: u32 | v > 0 } = a;
    let d: { v: u32 | v > 0 } = b;
    out.line(&c)?; // expect: 7
    out.line(&d)?; // expect: 2
    with_mut(&mut a, |x| *x = 9);
    out.line(&a)?; // expect: 9
    // A measure, and a name in the refinement.
    let n = 3;
    let mut x: { l: List<u8> | l.len() == n } = List::filled(n, 0);
    let mut y: { l: List<u8> | l.len() == n } = List::filled(n, 1);
    swap(&mut x, &mut y);
    out.line(&x[2])?; // expect: 1
    // An enum's type argument's refinement.
    let mut o: Option<{ v: u8 | v < 10 }> = Some(3);
    let mut p: Option<{ v: u8 | v < 10 }> = None;
    swap(&mut o, &mut p);
    let q: Option<{ v: u8 | v < 10 }> = p;
    out.line(&q.unwrap_or(0))?; // expect: 3
    // Fields.
    let mut band = Band { rows: 2, now: List::filled(2, 0), next: List::filled(2, 5) };
    flip(&mut band);
    out.line(&band.now.len())?; // expect: 2
    let mut s = Pos { n: 4 };
    let mut t = Pos { n: 6 };
    swap(&mut s.n, &mut t.n);
    out.line(&s.n)?; // expect: 6
    Ok(())
}
