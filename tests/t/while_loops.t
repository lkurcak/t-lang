// `while` loops, with mutable variables to count with.

fn fib(n: i64) -> i64 with panic {
    let mut a = 0;
    let mut b = 1;
    let mut i = 0;
    while i < n {
        let t = a + b;
        a = b;
        b = t;
        i += 1;
    }
    a
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&fib(10))?;             // expect: 55
    let mut n: i64 = 0;
    while n < 3 { n += 1; }
    out.line(&n)?;                   // expect: 3
    let mut never = true;
    while false { never = false; }
    out.line(&never)?;          // expect: true
    Result::Ok(())
}
