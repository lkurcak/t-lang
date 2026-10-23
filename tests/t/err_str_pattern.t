// Strings aren't patterns yet: compare them with `==`.
fn main() {
    match "a" {
        "a" => {},
        _ => {},
    }
}
// error: a string literal can't be a pattern yet; compare with `==`
