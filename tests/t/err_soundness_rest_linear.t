// `..` is the `_`s it stands for: a linear value under one is thrown away,
// which is an error, as it is under a `_`.
struct Fd { n: u8 }

fn close(fd: Fd) -> u8 {
    let Fd { n } = fd;
    n
}

enum Two {
    Both(Fd, Fd),
}

fn f(t: Two) -> u8 {
    match t {
        Two::Both(a, ..) => close(a),
    }
}

fn main() {
    let _ = f(Two::Both(Fd { n: 1 }, Fd { n: 2 }));
}
// error: this pattern throws away `t.1`
