// The same of a list's element, through `index_mut`.
use alloc::list::List;

#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn main() with alloc {
    let mut l: List<{ s: S | s.a < s.b }> = List::new();
    l.push(S { a: 1, b: 2 });
    l[0].a = 100;
    let _ = l;
}
// error: can't prove `s.a < s.b` for `l[0]`, once `l[0].a` is written
