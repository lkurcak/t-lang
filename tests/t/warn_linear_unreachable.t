// After a call that never returns, nothing is dropped, so nothing needs
// using up: the code after it is unreachable, and only warned about.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn die() -> ! {
    loop {}
}

fn give_up(fd: Fd) -> int {
    die();
    close(fd) // warning: unreachable code
}

fn main() {
    if 1 > 2 {
        print(give_up(open(1)));
    }
}
