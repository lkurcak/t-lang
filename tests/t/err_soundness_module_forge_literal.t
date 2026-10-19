// Outside `fs`, an `Fd` can't be made up: only `open` makes them.
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
    let fd = fs::Fd { id: fs::Id { n: 1 }, raw: 3 };
    print(fs::close(fd));
}
// error: field `raw` of `Fd` is private
