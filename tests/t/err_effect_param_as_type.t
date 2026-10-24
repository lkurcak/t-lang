// An effect parameter stands for effects, not a type.
fn f<effect e>(x: Option<e>) {}

fn main() {}
// error: `e` is an effect parameter, not a type
