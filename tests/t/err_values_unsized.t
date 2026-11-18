// A slice has no size of its own: the reference says what's known of it
// (docs/85-values-in-predicates.md).
fn f(s: &mut [u8]) ensures *s == old(*s) {}

fn main() {}
// error: a refinement can't name `*s`, a `[u8]`, which has no size: name it by the reference's measures, `s.len()`
