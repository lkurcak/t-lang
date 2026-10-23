// A byte literal is a `u8`, in a pattern too.
fn f(n: u32) -> bool {
    match n {
        b'a' => true,
        _ => false,
    }
}

fn main() {}
// error: expected `u32`, found `u8`
