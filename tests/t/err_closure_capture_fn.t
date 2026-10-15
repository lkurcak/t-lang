// A function that only takes references of one lifetime can't be captured:
// the closure could outlive that lifetime.
fn outer<'a>(f: fn(&'a int) -> int, x: &'a int) -> int {
    let g = || f(x);
    g()
}

fn main() {}
// error: this closure can't capture `f`
