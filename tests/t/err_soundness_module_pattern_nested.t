// Inside a variant, by value, the pattern still takes the `Fd` apart.
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

enum Opt { Some(fs::Fd), None }

fn main() {
    let o = Opt::Some(fs::open(1));
    match o {
        Opt::Some(fs::Fd { .. }) => print(1i64),
        Opt::None => print(0i64),
    }
}
// error: can't take `Fd` apart here
