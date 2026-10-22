// Matching through a reference only looks, so that's fine; but then
// `fd` still has to be closed.
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
    let fd = fs::open(1);
    match &fd {
        fs::Fd { id, .. } => print(id.n),
    }
}
// error: `fd` is never used up
