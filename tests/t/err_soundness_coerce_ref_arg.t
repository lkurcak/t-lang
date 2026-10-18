// `&b` is a `&Box<int>`; only a reference that's already one is coerced,
// not a `&` written there.
fn get(r: &int) -> &int { r }

fn main() {
    let b = Box::new(1);
    let r = get(&b);
    print(*r);
}
// error: expected `int`, found `Box<int>`
