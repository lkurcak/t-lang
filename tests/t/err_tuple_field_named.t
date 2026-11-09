// A tuple's fields have no names.
fn main() {
    let p = (1i64, true);
    let _x = p.x;
}
// error: `(i64, bool)` has no field `x`
