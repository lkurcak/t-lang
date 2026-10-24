// `?` passes an error up unchanged, so it can't mix two sinks' errors in
// one function: `B::Error` isn't `A::Error`.
use core::fmt::Write;

fn tee<A: Write, B: Write>(a: &mut A, b: &mut B, s: &str) -> Result<(), A::Error>
with {A::e, B::e}
{
    a.write_str(s)?;
    b.write_str(s)?;
    Result::Ok(())
}

fn main() {}
// error: `?` can't return early here: `tee` returns `Result<(), A::Error>`, so a `?` in it must stop with `Result<!, A::Error>`, and this one stops with `Result<!, B::Error>`
