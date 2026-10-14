// Nothing at a call could say what `T` is.
fn f<T>(x: int) -> int { x }
fn main() { print(f(1)); }
// error: type parameter `T` is never used in the signature of `f`
