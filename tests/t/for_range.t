// `for` over a range: `a..b` counts from `a` up to, not including, `b`. A
// range whose start isn't before its end is empty.

fn sum_to(n: i64) -> i64 with panic {
    let mut s = 0;
    for i in 0..n {
        s = s + i;
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&sum_to(5))?; // expect: 10
    out.line(&sum_to(0))?; // expect: 0
    out.line(&sum_to(-3))?; // expect: 0

    // `break` and `continue`, and labels.
    let mut found: i64 = 0;
    for i in 1..100 {
        if i % 7 != 0 {
            continue;
        }
        found = i;
        break;
    }
    out.line(&found)?; // expect: 7
    let mut pairs: i64 = 0;
    'outer: for i in 0..10i64 {
        for j in 0..10 {
            if i * j > 6 {
                continue 'outer;
            }
            if i == 5 {
                break 'outer;
            }
            pairs = pairs + 1;
        }
    }
    out.line(&pairs)?; // expect: 26

    // Precedence: arithmetic before `..`.
    let n: i64 = 2;
    let mut c: i64 = 0;
    for _ in n - 1..n * 3 {
        c = c + 1;
    }
    out.line(&c)?; // expect: 5

    // It doesn't overflow at the very end of `i64`.
    let big: i64 = 9223372036854775807;
    let mut last = 0;
    for i in big - 2..big {
        last = i;
    }
    out.line(&(last - big))?; // expect: -1

    // A range is a value, and its ends can be read.
    let r = 3..8i64;
    out.line(&(r.start() + r.end()))?; // expect: 11
    let mut r = r;
    out.line(&r.next().unwrap_or(0))?; // expect: 3
    out.line(&r.start())?; // expect: 4

    // A `for` is `()`: one can end a block.
    let mut f = || -> Result<(), std::io::Error> {
        for i in 0..3i64 { out.line(&(i * 10))?; }
        Result::Ok(())
    };
    f()
    // expect: 0
    // expect: 10
    // expect: 20
}
