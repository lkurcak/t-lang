// Integer arithmetic and operator precedence.

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(1i64 + 2 * 3))?;     // expect: 7
    out.line(&((1i64 + 2) * 3))?;   // expect: 9
    out.line(&(10i64 - 4 - 3))?;    // expect: 3
    out.line(&(17i64 / 5))?;        // expect: 3
    out.line(&(17i64 % 5))?;        // expect: 2
    out.line(&(-5i64 + 2))?;        // expect: -3
    out.line(&(--5i64))?;           // expect: 5
    out.line(&(1_000_000i64 * 3))?; // expect: 3000000
    Result::Ok(())
}
