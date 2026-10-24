// A `Result` thrown away as a statement loses any error in it: a warning,
// whose fix throws it away on purpose. Not a `Result<T, !>`, which has none.
use alloc::string::String;
use core::fmt::Write;

fn main(os: std::Os) with alloc {
    let mut out = os.stdout;
    out.line(&1i64); // expect: 1
    let mut s = String::new();
    s.put(&2i64);
    s.discard();
}
// warning: this `Result` is thrown away, and any error in it unseen
