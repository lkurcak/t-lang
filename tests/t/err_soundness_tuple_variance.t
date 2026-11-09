// A tuple is covariant: a `(&'static str, u8)` is a `(&'a str, u8)`, but
// not the other way round, or `t.0` would outlive what it points to.
fn longer<'a>(t: (&'a str, u8)) -> (&'static str, u8) { t }

fn main() {}
// error: lifetime may not live long enough
