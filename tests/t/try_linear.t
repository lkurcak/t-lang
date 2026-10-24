// What's in an `Ok` and what's in an `Err` can both be linear: `?` moves
// each to where it goes, once.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn open(n: i64) -> Result<Fd, Fd> with panic {
    if n > 0 { Result::Ok(Fd { n }) } else { Result::Err(Fd { n: -n }) }
}

fn twice(n: i64) -> Result<i64, Fd> with panic {
    let fd = open(n)?;
    Result::Ok(close(fd) * 2)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    match twice(4) {
        Result::Ok(v) => out.line(&v)?, // expect: 8
        Result::Err(fd) => out.line(&close(fd))?,
    }
    match twice(-5) {
        Result::Ok(v) => out.line(&v)?,
        Result::Err(fd) => out.line(&close(fd))?, // expect: 5
    }
    Result::Ok(())
}
