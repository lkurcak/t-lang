// Not dividing by zero isn't enough: the least of a signed type divided
// by -1 doesn't fit.
fn div<T: Int>(a: T, b: T) -> T with {} {
    if b != 0 { a / b } else { a }
}

fn main() {}
// error: `/` may panic, but `div` doesn't say `with panic`
