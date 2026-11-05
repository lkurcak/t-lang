// `fmt` is a method of both `Display` and `Debug`, and neither is in
// scope here: the error names both, as either could be meant.
struct Sink {}

fn main() {
    let s = Sink {};
    let n = 1i64;
    let _ = n.fmt(&mut s);
}
// error: `i64` has no method `fmt` in scope; traits `core::fmt::Debug` and `core::fmt::Display` have one: import the one meant
