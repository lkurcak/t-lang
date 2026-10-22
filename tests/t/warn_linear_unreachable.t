// After a call that never returns, nothing is dropped, so nothing needs
// using up: the code after it is unreachable, and only warned about.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn die() -> ! {
    loop {}
}

fn give_up(fd: Fd) -> i64 {
    die();
    close(fd) // warning: unreachable code
}

fn main() {
    if 1i64 > 2 {
        print(give_up(open(1)));
    }
}
