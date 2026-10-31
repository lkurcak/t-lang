// A `mut` parameter changes in the body: which of its values would it be?
fn f(mut n: usize) ensures n == 0 {
    n = 0;
}

fn main() {}
// error: a refinement can't name `n`, which is `mut`: it would be unclear which of its values it meant
