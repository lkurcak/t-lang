// What a branch proved of a value it had before the branch holds on
// that branch only: after the `if`, `l`'s elements are below `n` only
// where `n > 10` (step 99; a hole since 0.77.0).
fn first(z: [{ v: u8 | v < 3 }; 2]) -> { v: u8 | v < 3 } { z[0] }

fn f(n: u8, l: [{ x: u8 | x < 10 }; 2]) -> u8 {
    if n > 10 {
        let y: [{ v: u8 | v < n }; 2] = l;
        let _ = y;
    }
    let z: [{ v: u8 | v < n }; 2] = l;
    if n == 3 { first(z) } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(3, [5, 5]))?;
    Ok(())
}
// error: can't prove `v < n` for `z`
