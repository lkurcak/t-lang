// What an iteration's branch that can't be taken proves isn't known
// in another branch, or the next iteration (step 99).
fn first(z: [{ v: u8 | v < 3 }; 2]) -> { v: u8 | v < 3 } { z[0] }

fn f(l: [{ x: u8 | x < 10 }; 2]) -> u8 {
    let mut s = 0u8;
    for i in 0..3u8 {
        if i == 7 {
            let y: [{ v: u8 | v < 3 }; 2] = l;
            s = first(y);
        }
        if i == 2 {
            let z: [{ v: u8 | v < 3 }; 2] = l;
            s = first(z);
        }
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f([5, 5]))?;
    Ok(())
}
// error: can't prove `v < 3` for `z`
