// The closure holds a reference that lives for both `'a` and `'b`, which
// is neither: nothing says one outlives the other, so the type `either`
// returns can't say how long it borrows for.
fn either<'a, 'b>(c: bool, x: &'a i64, y: &'b i64) -> impl Fn() -> i64 + Discard {
    let r = if c { x } else { y };
    move || *r
}

fn main() {
    let n = 1;
    let m = 2;
    print(either(true, &n, &m)());
}
// error: borrows for a lifetime that isn't
