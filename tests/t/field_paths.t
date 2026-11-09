// A predicate names a field of a struct it can name, at any depth and
// through references and boxes, as it names a variable: `p.x`,
// `s.0.len()` (docs/56-field-paths-and-slices.md).

use alloc::boxed::Box;

#[derive(Copy, Discard)]
struct Point { x: u8, y: u8 }

#[derive(Copy, Discard)]
struct Line { from: Point, to: Point }

#[derive(Discard)]
struct Counter { n: u8 }

// A field of a parameter, in the result's refinement.
fn x_of(p: Point) -> { v: u8 | v == p.x } { p.x }

// A field of a parameter, in another parameter's.
fn below(p: Point, v: { v: u8 | v < p.y }) -> u8 { v }

// Deeper, and through a reference.
fn start_x(l: &Line) -> { v: u8 | v == l.from.x } { l.from.x }

// The binder's own fields.
fn diagonal() -> { p: Point | p.x == p.y && p.x == 3 } { Point { x: 3, y: 3 } }

// Through a box.
fn boxed_y(b: Box<Point>) -> { v: u8 | v == b.y } { b.y }

// Behind a call-scoped `&mut`: what it is at return, and with `old(..)`,
// at the call.
fn reset(c: &mut Counter) -> { v: u8 | v == old(c.n) } ensures c.n == 6 {
    let v = c.n;
    c.n = 6;
    v
}

// A struct's field's refinement names a field of a field before it.
#[derive(Copy, Discard)]
struct Box2 { corner: Point, w: { w: u8 | w <= 255 - corner.x } }

fn right(b: Box2) -> u8 { b.corner.x + b.w }

// A field and a measure of it: the first half of a slice's `split_at`.
fn halves(s: &[u8], mid: usize) -> usize {
    match s.split_at(mid) {
        Some(sp) => {
            let n: { n: usize | n == mid } = sp.0.len();
            let all: { a: usize | a == s.len() } = n + sp.1.len();
            all
        },
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let p = Point { x: 2, y: 5 };
    let a: { a: u8 | a == 2 } = x_of(p);
    out.line(&a)?;           // expect: 2
    out.line(&below(p, 4))?; // expect: 4
    let l = Line { from: p, to: Point { x: 9, y: 9 } };
    let b: { b: u8 | b == 2 } = start_x(&l);
    out.line(&b)?; // expect: 2
    let d = diagonal();
    let e: { e: u8 | e == 3 } = d.y;
    out.line(&e)?;                    // expect: 3
    out.line(&boxed_y(Box::new(p)))?; // expect: 5
    let mut c = Counter { n: 10 };
    let was: { w: u8 | w == 10 } = reset(&mut c);
    let n: { n: u8 | n == 6 } = c.n;
    out.line(&(was + n))?;                         // expect: 16
    out.line(&right(Box2 { corner: p, w: 250 }))?; // expect: 252
    let a: [u8; 5] = [1, 2, 3, 4, 5];
    out.line(&halves(a.as_slice(), 2))?; // expect: 5
    // A `let`'s refinement, and a loop's invariant, name fields too.
    let q: { q: u8 | q < p.y } = 4;
    let mut i = 0u8;
    while i < q invariant i <= p.y {
        i = i + 1;
    }
    out.line(&i)?; // expect: 4
    Ok(())
}
