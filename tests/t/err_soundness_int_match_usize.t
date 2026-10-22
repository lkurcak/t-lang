// A `usize` may be 64 bits wide, so covering 32 bits doesn't cover it.
fn f(n: usize) -> bool {
    match n {
        0..=4294967295 => true,
    }
}

fn main() {}
// error: non-exhaustive `match`: `4294967296..` not covered
