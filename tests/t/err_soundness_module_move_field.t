// Moving its pub field out takes the `Fd` apart too: what's left is
// private, so nothing outside could use it up, and `i64` is `Discard`.
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

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = fs::open(1);
    let id = fd.id;
    out.line(&id.n)?;
    Result::Ok(())
}
// error: can't take `Fd` apart here: it has private fields and isn't `Discard`, so only `fs` can
