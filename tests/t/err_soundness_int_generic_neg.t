// An absolute value that's right for unsigned types and wrong for signed
// ones: `0 - x` of the least `i8` doesn't fit.
fn abs<T: Int>(x: T) -> T {
    if x < 0 { 0 - x } else { x }
}

fn main() {}
// error: `-` may panic, but `abs` doesn't say `with panic`
