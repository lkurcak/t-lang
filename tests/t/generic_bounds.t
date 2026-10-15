// Bounds on type parameters: `T: 'a` says a `T` lives at least as long
// as `'a`, so a `&'a T` makes sense, and so does keeping a `T` for `'a`.

fn need<'a, T: 'a>(x: &'a int, _t: T) -> &'a int { x }

fn caller<'a, T: 'a>(x: &'a int, t: T) -> &'a int { need(x, t) }

// A `&'a T` in the signature says it without a bound.
fn implied<'a, T>(x: &'a T, y: &'a int) -> &'a int { need(y, x) }

fn main() {
    let n = 1;
    print(*caller(&n, true));   // expect: 1
    print(*implied(&false, &n)); // expect: 1
}
