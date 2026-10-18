// `return` drops what's in scope.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn f(early: bool) -> int {
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
