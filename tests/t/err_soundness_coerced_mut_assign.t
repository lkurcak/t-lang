// `&mut b` used as a `&int` keeps `b` borrowed as long as the result is
// used.
fn get(r: &int) -> &int { r }

fn main() with {alloc, panic} {
    let mut b = Box::new(1);
    let r = get(&mut b);
    b = Box::new(2);
    print(*r);
}
// error: cannot assign to `b` while `b` is mutably borrowed
