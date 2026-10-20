// Matching through a reference only looks, so that's fine; but then
// `fd` still has to be closed.
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
    match &fd {
        fs::Fd { id, .. } => print(id.n),
    }
}
// error: `fd` is never used up
