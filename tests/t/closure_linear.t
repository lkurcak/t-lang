// A closure that owns a value that must be used up is called once, and
// uses it up.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn later<F: FnOnce() -> i64>(f: F) -> i64 { f() }

fn main() {
    let fd = Fd { n: 3 };
    let c = || close(fd);
    print(later(c));                // expect: 3
    let fd = Fd { n: 4 };
    let c = move || close(fd);
    print(c());                     // expect: 4
}
