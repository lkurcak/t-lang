// A private field can't be read outside its module, even through a
// reference.
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

fn peek(fd: &fs::Fd) -> int { fd.raw }

fn main() { print(fs::close(fs::open(1))); }
// error: field `raw` of `Fd` is private
