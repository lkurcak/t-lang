// What a function ensures of a field behind a `&mut` it's lent is proved
// of what it leaves there.
#[derive(Discard)]
struct C { n: u8 }

fn f(c: &mut C) ensures c.n == 3 {
    if c.n == 3 {
        return;
    }
    c.n = 4;
}

fn main() {}
// error: can't prove `c.n == 3` for what `f` ensures
