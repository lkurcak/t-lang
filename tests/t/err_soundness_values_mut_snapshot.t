// A `&mut` stays the same value whatever is written through it, so a
// callee can promise to give back `old(*x)` and still write through it: the
// caller learns which reference it gets, not what was behind it at the call
// (lemma V5 of docs/85-values-in-predicates.md). The interpreter caught
// this while the step was written.
use core::mem::swap;

fn pick<'a>(x: &mut &'a mut u8, y: &'a mut u8) -> { r: &'a mut u8 | r == old(*x) } {
    let mut t = y;
    swap(x, &mut t);
    *t = 9;
    t
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    let mut a: u8 = 1;
    let mut b: u8 = 2;
    let mut ra = &mut a;
    *ra = 1;
    let v: { v: u8 | v == 1 } = *pick(&mut ra, &mut b);
    out.line(&v)?;
    Ok(())
}
// error: can't prove `v == 1` for `v`
