// An intrinsic as a value is shown where it was made a value, as one that
// blames its caller is: here core's `panic`.
fn call(f: fn(&str) -> ! with panic) with panic { f("through a value") }

fn main() {
    let p = core::panic::panic;
    call(p);
}
// error: blame_caller_value_intrinsic.t:6:13
