// Moving its pub field out takes the `Fd` apart too: what's left is
// private, so nothing outside could use it up, and `int` is `Discard`.
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
    let fd = fs::open(1);
    let id = fd.id;
    print(id.n);
}
// error: can't take `Fd` apart here: it has private fields and isn't `Discard`, so only `fs` can
