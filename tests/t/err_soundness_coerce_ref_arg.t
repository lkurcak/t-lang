// `&b` is a `&Box<i64>`; only a reference that's already one is coerced,
// not a `&` written there.
fn get(r: &i64) -> &i64 { r }

fn main() {
    let b = Box::new(1);
    let r = get(&b);
    print(*r);
}
// error: expected `i64`, found `Box<{integer}>`
