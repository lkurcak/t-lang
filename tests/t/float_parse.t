// Text as a float: the nearest, ties to even, rounded once, from the
// decimal, of each type.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let ok: [&str; 12] = [
        "2.5",
        "-0.0",
        "1e-3",
        ".5",
        "1.",
        "+7",
        "1E5",
        "inf",
        "-Infinity",
        "NaN",
        "0.1",
        "1e400",
    ];
    for s in ok {
        match s.parse::<f64>() {
            Ok(x) => out.line(&x)?,
            Err(e) => out.line(&e)?,
        }
    }
    // expect: 2.5
    // expect: -0.0
    // expect: 0.001
    // expect: 0.5
    // expect: 1.0
    // expect: 7.0
    // expect: 100000.0
    // expect: inf
    // expect: -inf
    // expect: NaN
    // expect: 0.1
    // expect: inf
    let bad: [&str; 5] = ["", "1e", "1.2.3", "e5", "0x10"];
    for s in bad {
        match s.parse::<f64>() {
            Ok(x) => out.line(&x)?,
            Err(e) => out.line(&e)?,
        }
    }
    // expect: cannot parse float from empty string
    // expect: invalid float literal
    // expect: invalid float literal
    // expect: invalid float literal
    // expect: invalid float literal
    // Halfway between two `f64`s, 2^53 + 1 goes to the even one; a little
    // more, to the one above.
    out.line(&"9007199254740993".parse::<f64>().unwrap_or(0.0))?; // expect: 9007199254740992.0
    out.line(&"9007199254740993.0000000000000000001".parse::<f64>().unwrap_or(0.0))?; // expect: 9007199254740994.0
    out.line(&"16777217".parse::<f32>().unwrap_or(0.0))?;                     // expect: 16777216.0
    out.line(&"1.00000017881393432617187499".parse::<f32>().unwrap_or(0.0))?; // expect: 1.0000001
    Ok(())
}
