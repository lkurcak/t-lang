fn yes(s: Option<&str>) -> bool {
    match s {
        Option::Some("y") => true,
        Option::None => false,
    }
}
fn main() {
    let _ = yes(Option::None);
}
// error: non-exhaustive `match`: `Option::Some(_)` not covered
