// A struct field whose refinement names another field, reached through a
// struct pattern: its refinement is the struct's, in the logic, which
// nothing proves again after a write through `a`; so as `&mut s.o` can't
// be, `a` can't be bound by `&mut`.
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
// error: can't bind `a` by `&mut`: field `o` of `S` has a refinement, which a write through the reference could break
