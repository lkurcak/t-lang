// An alternative of a different length says only what all of them share:
// here, at least two bytes, not three.
fn third(s: &str) -> u8 {
    match s {
        "abc" | "xy" => s.as_bytes()[2],
        _ => 0,
    }
}

fn main() {
    let _ = third("xy");
}
// error: can't prove `i < self.len()` for argument `i` of `index`
