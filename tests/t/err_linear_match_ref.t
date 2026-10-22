// Matching through a reference only looks: the value is still there.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let fd = open(1);
    match &fd {
        Fd { n } => print(*n),
    }
}
// error: `fd` is never used up
