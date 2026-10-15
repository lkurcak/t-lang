// `slot` can only hold references that live for `'a`; `v`'s may not.
fn store<'a>(slot: &mut &'a int, v: &int) { *slot = v; }

fn main() {}
// error: lifetime may not live long enough: this needs the lifetime of `v` to outlive `'a`
