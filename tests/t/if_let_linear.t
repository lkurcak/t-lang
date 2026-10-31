// Matching a linear value with `if let`, `while let` or `let .. else` uses
// it up on every path: the `else` takes it apart to the variant it is, and
// a `None` has nothing in it to throw away.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn close_any(o: Option<Fd>) -> i64 {
    if let Option::Some(fd) = o { close(fd) } else { 0 }
}

fn first(r: Result<Fd, i64>) -> i64 {
    let Result::Ok(fd) = r else { return -1 };
    close(fd)
}

fn open(i: i64) -> Option<Fd> {
    if i < 3 { Option::Some(Fd { n: i + 1 }) } else { Option::None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&close_any(Option::Some(Fd { n: 3 })))?; // expect: 3
    out.line(&close_any(Option::None))?; // expect: 0
    out.line(&first(Result::Ok(Fd { n: 4 })))?; // expect: 4
    out.line(&first(Result::Err(9)))?; // expect: -1
    let mut i = 0;
    let mut sum = 0;
    while let Option::Some(fd) = open(i) {
        sum = sum + close(fd);
        i = i + 1;
    }
    out.line(&sum)?; // expect: 6
    Result::Ok(())
}
