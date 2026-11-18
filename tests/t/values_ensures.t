// A function can say which value it leaves behind a `&mut`, or gives
// back: `*a == old(*b)`. Its callers then know of that value what they
// knew of the one it is (docs/85-values-in-predicates.md).
use alloc::list::List;
use core::mem::swap;

#[derive(Copy, Discard)]
struct Pair {
    lo: u8,
    hi: u8,
}

/// Exchanges what `a` and `b` point to, as `swap` does.
fn exchange(a: &mut Pair, b: &mut Pair) ensures *a == old(*b), *b == old(*a) { swap(a, b); }

/// Each of three gets the next one's value.
fn rotate<T>(a: &mut T, b: &mut T, c: &mut T) ensures *a == old(*b), *b == old(*c), *c == old(*a) {
    swap(a, b);
    swap(b, c);
}

/// One more, of an integer behind a reference.
fn bump(r: { r: &mut u8 | *r < 255 }) ensures *r == old(*r) + 1 { *r = *r + 1; }

/// Copies a pair, field by field: the same value.
fn copy_into(dst: &mut Pair, src: &Pair) ensures *dst == *src {
    *dst = Pair { lo: src.lo, hi: src.hi };
}

/// One of the two.
fn pick(a: Pair, b: Pair, first: bool) -> { r: Pair | r == a || r == b } {
    if first { a } else { b }
}

fn below(n: { n: u8 | n < 8 }) -> u8 { n }

fn other(a: &Pair, b: { b: &Pair | *b != *a }) -> u8 { b.lo }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut p = Pair { lo: 1, hi: 2 };
    let mut q = Pair { lo: 3, hi: 4 };
    exchange(&mut p, &mut q);
    out.line(&below(p.lo))?; // expect: 3
    let mut x: List<u8> = List::new();
    let mut y: List<u8> = List::new();
    y.push(5);
    let mut z: List<u8> = List::new();
    z.push(6);
    z.push(7);
    rotate(&mut x, &mut y, &mut z);
    out.line(&x[0])?; // expect: 5
    out.line(&y[1])?; // expect: 7
    let mut n: u8 = 5;
    bump(&mut n);
    bump(&mut n);
    out.line(&below(n))?; // expect: 7
    let mut d = Pair { lo: 0, hi: 0 };
    copy_into(&mut d, &q);
    out.line(&below(d.hi))?; // expect: 2
    let r = pick(p, q, false);
    out.line(&below(r.lo))?; // expect: 1
    out.line(&other(&p, &q))?; // expect: 1
    Ok(())
}
