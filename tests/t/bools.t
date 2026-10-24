// Booleans, comparisons and short-circuiting logic.

fn b(x: bool) -> i64 { if x { 1 } else { 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&b(1i64 < 2))?;                    // expect: 1
    out.line(&b(2i64 <= 1))?;                   // expect: 0
    out.line(&b(true && !false))?;           // expect: 1
    out.line(&b(false || 3i64 == 3))?;          // expect: 1
    out.line(&b(true != false))?;            // expect: 1
    out.line(&b(() == ()))?;                 // expect: 1
    // `||` binds looser than `&&`: this is `true || (false && false)`.
    out.line(&b(true || false && false))?;   // expect: 1
    // Short-circuit: the division by zero never runs.
    out.line(&b(false && 1i64 / 0 == 0))?;      // expect: 0
    Result::Ok(())
}
