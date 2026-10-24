// An effect parameter stands for any effects, so it has no bounds.
fn f<effect e: Discard>() {}

fn main() {}
// error: an effect parameter has no bounds: it stands for any effects
