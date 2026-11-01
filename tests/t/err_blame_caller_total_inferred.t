// The same for a function whose effects are inferred: none of them can
// abort.
#[blame_caller]
fn id(n: u8) -> u8 { n }

fn main() { let _ = id(1); }
// error: `#[blame_caller]` on a function that can't abort
