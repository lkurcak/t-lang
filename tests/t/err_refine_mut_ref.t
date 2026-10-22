// Through a `&mut i64`, anything could be written to `i`, which would break
// its refinement.
fn zero(r: &mut i64) { *r = 0; }

fn main() {
    let mut i: { v: i64 | v > 0 } = 1;
    zero(&mut i);
    print(i);
}
// error: can't borrow `i` as `&mut`: its type has a refinement, which a write through the reference could break
