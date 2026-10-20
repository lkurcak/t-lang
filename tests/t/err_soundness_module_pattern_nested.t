// Inside a variant, by value, the pattern still takes the `Fd` apart.
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

enum Opt { Some(fs::Fd), None }

fn main() {
    let o = Opt::Some(fs::open(1));
    match o {
        Opt::Some(fs::Fd { .. }) => print(1),
        Opt::None => print(0),
    }
}
// error: can't take `Fd` apart here
