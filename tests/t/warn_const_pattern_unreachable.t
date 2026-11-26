// After a `_`, a const's arm is never reached.
const N: u8 = 3;

fn f(x: u8) -> u8 {
    match x {
        _ => 0,
        N => 1,
// warning: unreachable
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(3))?; // expect: 0
    Ok(())
}
