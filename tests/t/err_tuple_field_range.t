// A pair has fields `0` and `1`, no more.
fn main() {
    let p = (1i64, true);
    let _x = p.2;
}
// error: `(i64, bool)` has no field `2`
