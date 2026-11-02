// A const in a refinement is its value; one the checker can't tell the
// value of is what its type says, the same wherever it's named. The
// interpreter checks each refinement with the value.
const LOW: i64 = -10;
const DEBUG: bool = false;

fn far() -> { n: u32 | n > 100 } { 1000 }

// What the logic knows of it: only that it's over 100.
const LIMIT: { n: u32 | n > 100 } = far();

fn above(x: { x: i64 | x > LOW }) -> i64 { x }

fn clamp(x: u32) -> { y: u32 | y <= LIMIT } {
    if x > LIMIT { LIMIT } else { x }
}

// Proved from `LIMIT > 100` alone, with no `panic` to fall back on.
fn headroom() -> u32 with {} { LIMIT - 100 }

fn check(b: { b: bool | b == DEBUG }) -> bool { b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&above(-9))?;      // expect: -9
    out.line(&clamp(5000))?;    // expect: 1000
    out.line(&clamp(50))?;      // expect: 50
    out.line(&headroom())?;     // expect: 900
    out.line(&check(false))?;   // expect: false
    Result::Ok(())
}
