// Moving out one value leaves a tuple that can't be used whole.
use alloc::string::String;

fn take(_t: (String, String)) {}

fn main() with alloc {
    let t = (String::from("a"), String::from("b"));
    let _a = t.0;
    take(t);
}
// error: use of partly moved value `t`: `t.0` was moved out of it
