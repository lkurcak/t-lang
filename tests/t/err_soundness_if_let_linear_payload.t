// The `else` of an `if let` can't name what's in an `Err`, so it would be
// thrown away there: only allowed if it's `Discard`.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn f(r: Result<i64, Fd>) -> i64 {
    if let Result::Ok(n) = r { n } else { 0 }
}

fn main() {
    let _ = close(Fd { n: f(Result::Ok(1)) });
}
// error: if the pattern of this `if let` doesn't match, it throws away `r.0`
