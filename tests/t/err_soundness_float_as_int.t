// A float isn't in the logic, so converting one to an integer is never
// proved: it may be a NaN, an infinity, have a fraction or be out of range.
fn f(x: f64) -> i32 with {} { x as i32 }
// error: `as` may panic, but `f` doesn't say `with panic`

fn main() {}
