// Counting through a range can't panic: `Range::next` checks before it
// adds one. The sum can.
fn sum(n: i64) -> i64 with panic {
    let mut s = 0;
    for i in 0..n {
        s = s + i;
    }
    s
}

fn count(n: i64) -> bool {
    for _i in 0..n {
        return true;
    }
    false
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&sum(4))?; // expect: 6
    out.line(&count(3))?; // expect: true
    Result::Ok(())
}
