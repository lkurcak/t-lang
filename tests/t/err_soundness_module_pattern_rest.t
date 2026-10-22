// Taking an `Fd` apart by a pattern outside `fs` would throw it away
// without closing it, though it isn't `Discard`.
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

fn main() {
    let fs::Fd { .. } = fs::open(1);
}
// error: can't take `Fd` apart here: it has private fields and isn't `Discard`, so only `fs` can
