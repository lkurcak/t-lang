// What a reference points to may be a variable holding a closure that
// captures, so it can't be returned.
fn get(r: &fn(int) -> int) -> fn(int) -> int { *r }

fn main() {}
// error: cannot return this function: it's read through a reference
