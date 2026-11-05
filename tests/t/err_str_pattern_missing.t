// No set of string literals covers every string.
fn yes(s: &str) -> bool {
    match s {
        "y" | "yes" => true,
        "n" | "no" => false,
    }
}
fn main() {
    let _ = yes("y");
}
// error: non-exhaustive `match`: `_` not covered
