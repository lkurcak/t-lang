// `as` gives the same number: where every value of the type converted is
// one of the other's, it can't panic, and is proved not to.
fn wide(a: i32, b: u8, c: f32, d: u32) -> f64 with {} { (a as f64) + (b as f64) + (c as f64) + (d as f64) }

fn narrow(a: i16, b: u8) -> f32 with {} { (a as f32) + (b as f32) }

/// A wider integer, where its refinement says it's at most 2^53 either
/// way, which every integer that far from 0 is.
fn exact(n: { n: i64 | n >= -9007199254740992 && n <= 9007199254740992 }) -> f64 with {} { n as f64 }

/// And to an `f32`, at most 2^24.
fn small(n: { n: usize | n <= 16777216 }) -> f32 with {} { n as f32 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&wide(-2147483648, 255, 0.5, 4294967295))?; // expect: 2147483902.5
    out.line(&narrow(-32768, 255))?; // expect: -32513.0
    out.line(&exact(-9007199254740992))?; // expect: -9007199254740992.0
    out.line(&small(16777216))?; // expect: 16777216.0
    // From a float, the same integer, if it's one.
    out.line(&(-128.0f64 as i8))?; // expect: -128
    out.line(&(4294967295.0f64 as u32))?; // expect: 4294967295
    out.line(&(9223372036854775807i64 as f32 == 9223372036854775808.0))?;
    Ok(())
}
// error: runtime error: `as` failed: 9223372036854775807 isn't an `f32` exactly
