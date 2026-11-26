// What one arm of a `match` proved isn't known after it (step 99).
fn first(z: [{ v: u8 | v < 3 }; 2]) -> { v: u8 | v < 3 } { z[0] }

fn f(n: u8, l: [{ x: u8 | x < 10 }; 2]) -> u8 {
    match n {
        11 => {
            let y: [{ v: u8 | v < n }; 2] = l;
            let _ = y;
        }
        _ => {}
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
