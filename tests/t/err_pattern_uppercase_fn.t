// A function can't be a pattern.
#[allow(non_snake_case)]
fn Five() -> i64 { 5 }

fn main() {
    match 5 {
        Five => {}
// error: `Five` is a function: a pattern can't name one
        _ => {}
    }
}
