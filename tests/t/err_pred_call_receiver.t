// A measure is called of a name, or of a field of one: what a call gives
// isn't something a predicate can name.
fn s() -> &'static [u8] { b"ab".as_slice() }

fn f(x: { x: usize | x < s().len() }) -> usize { x }

fn main() {}
// error: a refinement can only call a measure of a name, or of a field of one
