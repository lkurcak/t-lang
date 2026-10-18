// The same for `&rb`, a `&&Box<int>`.
fn get(r: &int) -> &int { r }

fn main() {
    let b = Box::new(1);
    let rb = &b;
    let r = get(&rb);
    print(*r);
}
// error: expected `int`, found `&Box<int>`
