// A named `for` over values that aren't `Discard` can stop early once it
// has used up the rest itself: what's left is then nothing, which may be
// thrown away (docs/80, docs/84).
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    l.push(Fd { n: 3 });
    let mut closed: i64 = 0;
    for fd in it: l invariant closed <= 6 {
        if close(fd) == 1 {
            closed = 1;
            while let Some(rest) = it.next() invariant closed <= 6 {
                if close(rest) == 3 {
                    closed = 6;
                }
            }
            break;
        }
    }
    out.line(&closed)?; // expect: 6
    Ok(())
}
