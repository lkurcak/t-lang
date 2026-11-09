// A `match` on a tuple must cover every combination of its values.
fn f(a: bool, b: bool) -> u8 {
    match (a, b) {
        (true, _) => 1,
        (false, true) => 2,
    }
}

fn main() { let _ = f(true, false); }
// error: non-exhaustive `match`: `(false, false)` not covered
