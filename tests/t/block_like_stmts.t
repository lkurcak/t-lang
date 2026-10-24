// An `if`, `match` or block at the start of a statement is the whole
// statement, like in Rust.

fn note(_x: i64) {}

fn f(x: i64) -> i64 with panic {
    if x > 100 { note(x) }    // a statement of type `()`
    -x                        // the block's value: not `(if ..) - x`
}

fn g(x: i64) -> i64 with panic {
    // Parenthesize to use one in a larger expression.
    (if x > 0 { 10 } else { 20 }) - x
}

fn h(b: bool) -> i64 {
    match b { true => note(1), false => note(0) }
    { note(2) }
    if b { 3 } else { 4 }     // the block's value, so any type is fine
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(5))?;         // expect: -5
    out.line(&g(3))?;         // expect: 7
    out.line(&h(true))?;      // expect: 3
    if true { 1i64 } else { 2 }; // with `;` the value is discarded, any type
    Result::Ok(())
}
