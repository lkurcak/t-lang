// A tuple's element whose type is refined whole: a write to its field
// must keep the element's refinement. (It left `S { a: 100, b: 2 }` in
// `t.1` before step 66.)
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn main() {
    let mut t: (u8, { s: S | s.a < s.b }) = (0, S { a: 1, b: 2 });
    t.1.a = 100;
    let _ = t;
}
// error: can't prove `s.a < s.b` for `t.1`, once `t.1.a` is written
