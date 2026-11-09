// Tuples: `(a, b)` makes one, `t.0` reads a field, `(p, q)` matches one.
// A tuple is as a struct with fields `0`, `1`, .., every one exported.
use alloc::boxed::Box;

fn swap<A, B>(p: (A, B)) -> (B, A) {
    let (a, b) = p;
    (b, a)
}

fn div_rem(a: u32, b: { b: u32 | b > 0 }) -> (u32, u32) { (a / b, a % b) }

#[derive(Copy, Discard)]
struct Seg { ends: (i32, i32) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = (1i64, true);
    out.line(&p.0)?;                // expect: 1
    out.line(&p.1)?;                // expect: true
    let (q, r) = div_rem(17, 5);
    out.line(&q)?;                  // expect: 3
    out.line(&r)?;                  // expect: 2
    let s = swap(('x', 7u8));
    out.line(&s.0)?;                // expect: 7
    out.line(&s.1)?;                // expect: x
    // Nested, and written to field by field.
    let mut n: ((u8, u8), (u8, (u8, u8))) = ((1, 2), (3, (4, 5)));
    n.1.1.0 = 40;
    n.0 = (10, 20);
    out.line(&n.1.1.0)?;            // expect: 40
    out.line(&(n.0.0 + n.0.1))?;    // expect: 30
    // In a struct, an array, a box.
    let g = Seg { ends: (-1, 1) };
    out.line(&(g.ends.1 - g.ends.0))?; // expect: 2
    let a: [(u8, char); 2] = [(1, 'a'), (2, 'b')];
    out.line(&a[1].1)?;             // expect: b
    let b = Box::new((5u8, 6u8));
    out.line(&b.1)?;                // expect: 6
    // A tuple of `Copy` values is `Copy`.
    let c = p;
    out.line(&(c.0 + p.0))?;        // expect: 2
    // `==` compares field by field.
    out.line(&((1u8, 'a') == (1u8, 'a')))?; // expect: true
    out.line(&((1u8, 'a') != (1u8, 'b')))?; // expect: true
    // Parentheses around one value only group it.
    let one: u8 = (2 + 3) * 2;
    out.line(&one)?;                // expect: 10
    let unit: () = ();
    out.line(&f"{unit:?}")?;        // expect: ()
    Ok(())
}
