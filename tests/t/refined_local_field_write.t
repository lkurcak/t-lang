// A write to a field of a refined `let mut` that keeps its refinement is
// fine, and what's known after is what was written.
#[derive(Copy, Discard)]
struct P { a: usize, b: usize }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p: { p: P | p.a < p.b } = P { a: 1, b: 5 };
    p.a = 3;
    p.b += 1;
    let mut t: { t: (u8, u8) | t.0 <= t.1 } = (0, 9);
    t.0 = t.1;
    let a: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
    out.line(&a[p.b])?;        // expect: 6
    out.line(&(p.a + t.0 as usize))?; // expect: 12
    Ok(())
}
