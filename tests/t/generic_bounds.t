// Bounds on type parameters: `T: 'a` says a `T` lives at least as long
// as `'a`, so a `&'a T` makes sense, and so does keeping a `T` for `'a`.

fn need<'a, T: Discard + 'a>(x: &'a i64, _t: T) -> &'a i64 { x }

fn caller<'a, T: Discard + 'a>(x: &'a i64, t: T) -> &'a i64 { need(x, t) }

// A `&'a T` in the signature says it without a bound.
fn implied<'a, T>(x: &'a T, y: &'a i64) -> &'a i64 { need(y, x) }

fn main() {
    let n = 1;
    print(*caller(&n, true));   // expect: 1
    print(*implied(&false, &n)); // expect: 1
}
