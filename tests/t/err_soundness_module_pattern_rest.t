// Taking an `Fd` apart by a pattern outside `fs` would throw it away
// without closing it, though it isn't `Discard`.
mod fs {
    pub struct Fd { pub id: Id, raw: int }

    #[derive(Discard)]
    pub struct Id { pub n: int }

    pub fn open(n: int) -> Fd { Fd { id: Id { n }, raw: n } }

    pub fn close(fd: Fd) -> int with panic {
        let Fd { id, raw } = fd;
        id.n + raw
    }
}

fn main() {
    let fs::Fd { .. } = fs::open(1);
}
// error: can't take `Fd` apart here: it has private fields and isn't `Discard`, so only `fs` can
