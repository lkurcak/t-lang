// A variant has no type arguments of its own: they're its enum's.
fn main() {
    let x = None::<i64>;
// error: a variant has no type arguments of its own; write them after its enum: `Option::<i64>::None`
    let _ = x;
}
