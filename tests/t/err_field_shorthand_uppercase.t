// `Point { X }` would bind a variable `X`, which can't be (step 45).
#[allow(non_snake_case)]
#[derive(Discard)]
struct Point { X: i64 }

fn main() {
    let Point { X } = Point { X: 1 };
// error: a variable's name can't start with an uppercase letter
    let _ = X;
}
