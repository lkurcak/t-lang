// A borrow of a tuple's field keeps the tuple from moving while it lives.
use alloc::string::String;

fn take(_t: (String, i64)) {}

fn main() with alloc {
    let t = (String::from("a"), 1);
    let r = &t.0;
    take(t);
    let _n = r.len();
}
// error: cannot move out of `t` while `t.0` is borrowed
