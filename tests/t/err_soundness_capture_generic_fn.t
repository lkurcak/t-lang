// A closure can't capture a function value whose type mentions a type
// parameter: `T` may be a reference type, so `k` takes references of a
// certain lifetime only.
fn call<T: Copy>(x: T, k: fn(T) -> int) -> int {
    let g = || k(x);
    g()
}

fn deref(r: &int) -> int { *r }

fn main() {
    let n = 5;
    print(call(&n, deref));
}
// error: this closure can't capture `k`
