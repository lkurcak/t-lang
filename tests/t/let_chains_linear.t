// A linear value a chain's `let` takes is there in the conditions after
// it, and where one of them doesn't hold, nothing uses it up: so it must
// be used up before a condition that can fail, or after the last.
struct Fd { n: u8 }

fn close(fd: Fd) -> u8 {
    let Fd { n } = fd;
    n
}

fn open(n: u8) -> Option<Fd> {
    if n > 0 { Some(Fd { n }) } else { None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    if let Some(fd) = open(3) && let n = close(fd) && n > 2 {
        out.line(&n)?; // expect: 3
    }
    if let Some(0) = Some(1u8) && let Some(fd) = open(4) {
        out.line(&close(fd))?;
    } else if let Some(fd) = open(2) {
        out.line(&close(fd))?; // expect: 2
    }
    Ok(())
}
