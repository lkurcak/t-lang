// `Item` is an associated type: what it is is a type.
fn f<I: Iterator<Item = {alloc}> + Discard>(_i: I) {}

fn main() {}
// error: `Item` is an associated type, not an effect: say what it is with a type, as in `Item = i64`
