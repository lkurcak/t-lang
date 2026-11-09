// A tuple is `Copy` only if all its values are: this one moves.
use alloc::string::String;

fn main() with alloc {
    let t = (1u8, String::from("s"));
    let _a = t;
    let _b = t;
}
// error: use of moved value `t`
