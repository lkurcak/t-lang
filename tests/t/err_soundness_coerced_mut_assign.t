// `&mut b` used as a `&i64` keeps `b` borrowed as long as the result is
// used.
fn get(r: &i64) -> &i64 { r }

fn main() with alloc {
    let mut b = Box::new(1);
    let r = get(&mut b);
    b = Box::new(2);
    print(*r);
}
// error: cannot assign to `b` while `b` is mutably borrowed
