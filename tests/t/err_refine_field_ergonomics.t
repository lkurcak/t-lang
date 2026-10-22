// A pattern that binds a refined field by `&mut` borrows it.
#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main() {
    let mut p = Pos { n: 1 };
    match &mut p {
        Pos { n } => *n = 0,
    }
    print(p.n);
}
// error: can't bind `n` by `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
