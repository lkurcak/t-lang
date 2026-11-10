// A struct field whose refinement names another field, reached through a
// struct pattern: a write through `a` would have to keep `v < n`, which
// nothing would check, so the binding is refused.
#[derive(Discard)]
struct S { n: usize, o: Option<{ v: usize | v < n }> }

fn main() {
    let mut s = S { n: 3, o: Some(1) };
    match &mut s {
        S { o: Some(a), .. } => *a = 99,
        _ => {},
    }
    let _ = s;
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
