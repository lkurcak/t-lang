// A parameter's refinement of what's behind its `&mut` is what holds at
// the call: the function may change it, which nothing after counts on.
// The interpreter checked it each time the parameter was used, until step
// 85 (docs/85-values-in-predicates.md).
#[derive(Copy, Discard)]
struct Pair {
    lo: u8,
    hi: u8,
}

fn bump(p: &mut Pair) { p.lo = 2; }

fn go(x: { x: &mut Pair | x.lo == 1 }) -> u8 {
    bump(x);
    x.lo
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pair { lo: 1, hi: 0 };
    out.line(&go(&mut p))?; // expect: 2
    Ok(())
}
