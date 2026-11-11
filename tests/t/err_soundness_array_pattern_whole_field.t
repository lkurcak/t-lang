// An array's element bound by `&mut` by a pattern, whose type is refined
// whole: a write to its field must keep that, as the reference says it.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn main() {
    let mut arr: [{ s: S | s.a < s.b }; 2] = [S { a: 1, b: 2 }, S { a: 1, b: 2 }];
    match &mut arr {
        [s, _] => s.a = 100,
    }
    let _ = arr;
}
// error: can't prove `s.a < s.b` for `s`, once `s.a` is written
