// A branch that can't be taken proves anything; what it proves of a
// value isn't known anywhere else (step 99).
fn first(z: [{ v: u8 | v < 3 }; 2]) -> { v: u8 | v < 3 } { z[0] }

fn f(n: u8, l: [u8; 2]) -> u8 {
    if n > 5 && n < 3 {
        let y: [{ v: u8 | v < 3 }; 2] = l;
        let _ = y;
    }
    let z: [{ v: u8 | v < 3 }; 2] = l;
    first(z)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(0, [200, 200]))?;
    Ok(())
}
// error: can't prove `v < 3` for `z`
