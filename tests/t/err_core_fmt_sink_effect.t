// Writing to a sink does what the sink does, which a function generic over
// it has to say.
use core::fmt::Write;

fn greet<W: Write>(w: &mut W) -> Result<(), W::Error> { w.write_str("hi") }

fn main() {}
// error: `write_str` may have the effects `W::e`, but `greet` doesn't say `with W::e`
