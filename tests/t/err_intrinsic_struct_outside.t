// What an intrinsic type is, is what the checker trusts, so only the
// built-in libraries can declare one.
#[intrinsic]
struct Buf {}

fn main() {}
// error: only `core`, `alloc` and `std` can declare an intrinsic
