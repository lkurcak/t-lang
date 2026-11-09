// A tuple holding a reference to a local can't outlive the local.
fn make<'a>() -> (&'a i64, i64) {
    let x = 1;
    (&x, 2)
}

fn main() {}
// error: borrowed
