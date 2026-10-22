// All of a `u8` is 0 to 255, and 255 isn't covered.
fn f(b: u8) -> bool {
    match b {
        0..=254 => true,
    }
}

fn main() {}
// error: non-exhaustive `match`: `255` not covered
