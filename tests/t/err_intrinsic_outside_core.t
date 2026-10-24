// Only the built-in libraries can declare an intrinsic: what the checker can't check.
#[intrinsic]
fn magic<T>(x: &mut T);

fn main() {}
// error: only `core`, `alloc` and `std` can declare an intrinsic
