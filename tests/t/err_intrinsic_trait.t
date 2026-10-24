// Only `core` and `alloc` can say the checker supplies a trait's impls.
#[intrinsic]
trait Magic {}

fn main() {}
// error: only `core`, `alloc` and `std` can declare an intrinsic
