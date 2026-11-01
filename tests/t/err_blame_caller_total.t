// A function that can't abort has nothing to blame its caller for.
#[blame_caller]
fn id(n: u8) -> u8 with {} { n }

fn main() { let _ = id(1); }
// error: `#[blame_caller]` on a function that can't abort
