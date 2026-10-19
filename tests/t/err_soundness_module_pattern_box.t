// Through a box, too.
mod fs {
    pub struct Fd { pub id: Id, raw: int }

    #[derive(Discard)]
    pub struct Id { pub n: int }

    pub fn open(n: int) -> Fd { Fd { id: Id { n }, raw: n } }

    pub fn close(fd: Fd) -> int {
        let Fd { id, raw } = fd;
        id.n + raw
    }
}

fn main() {
    let b = Box::new(fs::open(1));
    let fs::Fd { id, .. } = *b;
    print(id.n);
}
// error: can't take `Fd` apart here
