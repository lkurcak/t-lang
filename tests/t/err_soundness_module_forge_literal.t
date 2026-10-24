// Outside `fs`, an `Fd` can't be made up: only `open` makes them.
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
    let fd = fs::Fd { id: fs::Id { n: 1 }, raw: 3 };
    out.line(&fs::close(fd))?;
    Result::Ok(())
}
// error: field `raw` of `Fd` is private
