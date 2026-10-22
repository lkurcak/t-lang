// A field's refinement names the field's value by its binder.
struct S { x: { v: i64 | v > 0 && x > 0 } }

fn main() {}
// error: a field's refinement names the field's own value by its binder, `v`, not `x`
