// In generic code only 0 to 127 is known to fit: 200 may not.
fn f<T: Int>(x: T) -> T {
    if x < 100 { x + 100 } else { x }
}

fn main() {}
// error: `+` may panic, but `f` doesn't say `with panic`
