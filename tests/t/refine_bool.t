// A `bool` can be refined too, and a `bool` variable remembers what it was
// computed from.
fn needs(b: { b: bool | b }) -> i64 {
    if b { 1 } else { 0 }
}

fn div(a: i64, d: { d: i64 | d != 0 && d != -1 }) -> i64 { a / d }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 5;
    let positive = x > 0;
    if positive {
        out.line(&div(10, x))?; // expect: 2
    }
    out.line(&needs(true))?; // expect: 1
    out.line(&needs(x == 5))?; // expect: 1
    Result::Ok(())
}
