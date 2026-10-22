// A `let mut` keeps its refinement: each assignment must have it.
fn main() with panic {
    let mut i: { v: i64 | v >= 0 } = 0;
    i = i - 1;
    print(i);
}
// error: can't prove `v >= 0` for `i`
