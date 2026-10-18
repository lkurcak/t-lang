// Matching through a reference only looks: the value is still there.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
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
