// `return` drops what's in scope.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn f(early: bool) -> i64 {
    let fd = open(1);
    if early {
        return 0;
    }
    close(fd)
}

fn main() {
    print(f(false));
}
// error: `fd` is never used up
