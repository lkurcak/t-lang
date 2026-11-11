// The same through a `&mut` of the struct: the struct field's refinement
// says something of what's in its `Some`, which a write through `a`
// would change, and nothing would prove again.
#[derive(Discard)]
struct S { n: usize, o: Option<{ v: usize | v < n }> }

fn f(s: &mut S) {
    match s {
        S { o: Some(a), .. } => *a = 99,
        _ => {},
    }
}

fn main() {
    let mut s = S { n: 3, o: Some(1) };
    f(&mut s);
}
// error: can't bind `a` by `&mut`: field `o` of `S` has a refinement, which a write through the reference could break
