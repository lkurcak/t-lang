// Nor written.
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
    let mut fd = fs::open(1);
    fd.raw = 5;
    print(fs::close(fd));
}
// error: field `raw` of `Fd` is private
