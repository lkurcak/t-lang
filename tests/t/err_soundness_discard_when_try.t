// And one that `?` returns early from.
use alloc::list::List;

struct Fd { n: i64 }

fn close(out: &mut std::io::Stdout, fd: Fd) -> Result<(), std::io::Error> {
    let Fd { n } = fd;
    out.line(&n)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    for fd in l {
        close(&mut out, fd)?;
    }
    Ok(())
}
// error: the `for` may stop before its iterator is used up, and drops it then: `IntoIter<Fd>` isn't `Discard`, only `{ it: IntoIter<Fd> | it.len() == 0 }` is
