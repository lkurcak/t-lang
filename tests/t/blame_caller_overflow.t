// An operator's panic in a `#[blame_caller]` function is its caller's.
#[blame_caller]
fn add(a: u8, b: u8) -> u8 with panic { a + b }

fn main() {
    let _ = add(1, 2);
    let _ = add(200, 100);
}
// error: blame_caller_overflow.t:7:13
