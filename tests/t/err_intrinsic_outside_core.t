// Only the built-in libraries can declare an intrinsic: what the checker can't check.
#[intrinsic]
fn magic<T>(x: &mut T);

fn main() {}
// error: only `core` and `alloc` can declare an intrinsic
