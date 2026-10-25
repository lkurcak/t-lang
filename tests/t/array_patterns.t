// `[a, b, c]` takes an array apart, each value moved into its own binding:
// the way to use up an array of linear values one by one.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn first_true(a: [bool; 2]) -> i64 {
    match a {
        [true, _] => 0,
        [false, true] => 1,
        [false, false] => 2,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fds = [Fd { n: 3 }, Fd { n: 4 }];
    let [a, b] = fds;
    out.line(&close(b))?;               // expect: 4
    out.line(&close(a))?;               // expect: 3
    out.line(&first_true([false, true]))?; // expect: 1
    let [x, y, z] = [1, 2, 3];
    out.line(&(x + y + z))?;            // expect: 6
    Result::Ok(())
}
