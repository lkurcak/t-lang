// A private field can't be read outside its module, even through a
// reference.
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

fn peek(fd: &fs::Fd) -> i64 { fd.raw }

fn main() { print(fs::close(fs::open(1))); }
// error: field `raw` of `Fd` is private
