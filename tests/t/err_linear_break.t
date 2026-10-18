// So does `break`.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    loop {
        let fd = open(1);
        if 1 < 2 {
            break;
        }
        print(close(fd));
    }
}
// error: `fd` is never used up
