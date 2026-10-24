// `Fd` has a private field, so only `fs` can make one or take one apart:
// outside, it can be moved, borrowed and handed back. Its pub field can be
// read through a place.
use alloc::string::String;
use core::fmt::Write;

mod fs {
    use alloc::boxed::Box;

    pub struct Fd { pub id: i64, raw: i64 }

    pub fn open(id: i64) -> Fd with panic { Fd { id, raw: id * 100 } }

    pub fn close(fd: Fd) -> i64 with panic {
        let Fd { id, raw } = fd;
        id + raw
    }

    impl Fd {
        pub fn raw(&self) -> i64 { self.raw }
    }

    // A `Discard` one can be taken apart anywhere its fields can be seen:
    // the module said throwing its parts away is fine.
    #[derive(Discard)]
    pub struct Token { pub n: Box<i64>, secret: i64 }

    pub fn token(n: i64) -> Token with {alloc, panic} { Token { n: Box::new(n), secret: 0 } }
}

fn keep(fd: &fs::Fd) -> i64 with panic { fd.id + fd.raw() }

fn main(os: std::Os) -> Result<(), std::io::Error> with {panic, alloc} {
    let mut out = String::new();
    let fd = fs::open(2);
    out.line(&fd.id);         // expect: 2
    out.line(&keep(&fd));     // expect: 202
    let moved = fd;
    out.line(&fs::close(moved)); // expect: 202

    let t = fs::token(5);
    let n = t.n;
    out.line(&(*n));            // expect: 5
    let fs::Token { n, .. } = fs::token(6);
    out.line(&(*n));            // expect: 6
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
