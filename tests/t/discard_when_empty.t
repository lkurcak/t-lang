// A list of linear values may be dropped where the checker knows it's
// empty: made empty, emptied by `pop`, or overwritten while empty
// (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn drain(mut l: List<Fd>) -> i64 with panic {
    let mut total = 0;
    while let Some(fd) = l.pop() {
        total = total + close(fd);
    }
    total
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let _none: List<Fd> = List::new();
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 10 });
    l.push(Fd { n: 20 });
    out.line(&drain(l))?; // expect: 30
    let mut m: List<Fd> = List::with_capacity(4);
    m = List::new();
    m.push(Fd { n: 1 });
    match m.pop() {
        Some(fd) => out.line(&close(fd))?, // expect: 1
        None => {}
    }
    if m.is_empty() {
        out.line(&"empty")?; // expect: empty
        return Ok(());
    }
    m.free_with(|fd| { close(fd); });
    Ok(())
}
