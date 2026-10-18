// `&mut b` coerced to a `&` is still a `&mut` borrow of `b` first, so `b`
// must be `mut`.
fn get(r: &int) -> int { *r }

fn main() {
    let b = Box::new(1);
    print(get(&mut b));
}
// error: cannot borrow `b` as mutable, as `b` isn't declared `mut`
