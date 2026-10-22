// So does `break`.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    loop {
        let fd = open(1);
        if 1i64 < 2 {
            break;
        }
        print(close(fd));
    }
}
// error: `fd` is never used up
