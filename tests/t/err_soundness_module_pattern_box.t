// Through a box, too.

use alloc::boxed::Box;

mod fs {
    pub struct Fd { pub id: Id, raw: i64 }

    #[derive(Discard)]
    pub struct Id { pub n: i64 }

    pub fn open(n: i64) -> Fd { Fd { id: Id { n }, raw: n } }

    pub fn close(fd: Fd) -> i64 with panic {
        let Fd { id, raw } = fd;
        id.n + raw
    }
}

fn main() with alloc {
    let b = Box::new(fs::open(1));
    let fs::Fd { id, .. } = *b;
    print(id.n);
}
// error: can't take `Fd` apart here
