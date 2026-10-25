// A match on an array must cover each combination of its values.
fn f(a: [bool; 2]) -> i64 {
    match a {
        [true, _] => 0,
        [false, true] => 1,
    }
}

fn main() {}
// error: non-exhaustive `match`: `[false, false]` not covered
