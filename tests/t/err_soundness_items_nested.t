// A bound saying an iterator's items are of its own type lets a function
// generic in it give back one of its items, a new iterator, which gives
// what that one does, not what the one it was given does.
#[derive(Discard)]
struct Tower { lvl: usize, cap: usize }

impl Tower {
    #[measure]
    fn lvl(&self) -> usize { self.lvl }

    #[measure]
    fn cap(&self) -> usize { self.cap }
}

impl Iterator for Tower {
    type Item = Tower;
    effect e = {};

    fn next(&mut self) -> Option<{ t: Tower | t.lvl() < old(self.cap()) }>
    ensures
        self.cap() == old(self.cap()),
    {
        if self.cap > 0 { Some(Tower { lvl: self.cap - 1, cap: 100 }) } else { None }
    }
}

fn deeper<X: Iterator<Item = X> + Discard>(mut x: X) -> X {
    match x.next() {
        Some(y) => y,
        None => x,
    }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    let t = deeper(Tower { lvl: 0, cap: 8 });
    for u in t {
        let _ = a[u.lvl()];
    }
}
// error: can't prove
