// The loop ends on an `Err`, whose `Fd` nothing can name.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn get(i: i64) -> Result<i64, Fd> {
    if i < 3 { Result::Ok(i) } else { Result::Err(Fd { n: i }) }
}

fn main() {
    let mut i = 0;
    while let Result::Ok(k) = get(i) {
        i = i + k + 1;
    }
    let _ = close(Fd { n: i });
}
// error: if the pattern of this `while let` doesn't match, it throws away part of the value it matches
