// A parameter is a variable, so its name can't start with an uppercase
// letter either (step 45): in an expression, such a name is a path.
fn double(N: i64) -> i64 {
// error: a variable's name can't start with an uppercase letter
    N * 2
}

fn main() {
    let _ = double(2);
}
