fn yes(s: Option<&str>) -> bool {
    match s {
        Some("y") => true,
        None => false,
    }
}
fn main() {
    let _ = yes(None);
}
// error: non-exhaustive `match`: `Option::Some(_)` not covered
