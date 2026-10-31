// The `else` returns while `fd` is still there to be used up.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn f(o: Option<i64>) -> i64 {
    let fd = Fd { n: 1 };
    let Option::Some(n) = o else { return 0 };
    n + close(fd)
}

fn main() {
    let _ = f(Option::None);
}
// error: `fd` goes out of scope here
