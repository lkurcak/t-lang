// A const parameter takes a length, not a type.
fn zeros<const N: usize>() -> [i64; N] { [0; N] }

fn main() {
    let _z = zeros::<i64>();
}
// error: this argument of `zeros` is a length, not a type
