// `Fd` has a private field, so only `fs` can make one or take one apart:
// outside, it can be moved, borrowed and handed back. Its pub field can be
// read through a place.
mod fs {
    pub struct Fd { pub id: int, raw: int }

    pub fn open(id: int) -> Fd { Fd { id, raw: id * 100 } }

    pub fn close(fd: Fd) -> int {
        let Fd { id, raw } = fd;
        id + raw
    }

    impl Fd {
        pub fn raw(&self) -> int { self.raw }
    }

    // A `Discard` one can be taken apart anywhere its fields can be seen:
    // the module said throwing its parts away is fine.
    #[derive(Discard)]
    pub struct Token { pub n: Box<int>, secret: int }

    pub fn token(n: int) -> Token { Token { n: Box::new(n), secret: 0 } }
}

fn keep(fd: &fs::Fd) -> int { fd.id + fd.raw() }

fn main() {
    let fd = fs::open(2);
    print(fd.id);         // expect: 2
    print(keep(&fd));     // expect: 202
    let moved = fd;
    print(fs::close(moved)); // expect: 202

    let t = fs::token(5);
    let n = t.n;
    print(*n);            // expect: 5
    let fs::Token { n, .. } = fs::token(6);
    print(*n);            // expect: 6
}
