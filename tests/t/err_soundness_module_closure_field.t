// A closure that takes the pub field by value would take the `Fd` apart
// as surely as moving it out does.
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
    let f = move || fd.id;
    out.line(&f().n)?;
    Result::Ok(())
}
// error: can't take `Fd` apart here
