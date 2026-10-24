// `Iterator::filter` keeps the values a predicate says `true` of. The
// predicate takes each by `&`, for as long as the call, of whatever
// lifetime that is: a closure meets that bound when the bound asks for it
// (`|x| *x % 2 == 0`) or when its annotations say so (`|x: &i64|`).
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    for x in (0..10).filter(|x| *x % 2i64 == 0) {
        out.line(&x)?;
    }
    // expect: 0
    // expect: 2
    // expect: 4
    // expect: 6
    // expect: 8

    let low = 3;
    let above = |x: &i64| *x > low;
    out.line(&(0..10).filter(above).count())?;   // expect: 6

    // One that changes what it captures: `FnMut`.
    let mut seen: i64 = 0;
    let n = (0..5i64)
        .filter(|x| {
            seen += 1;
            *x != 2
        })
        .fold(0, |acc, x| acc + x);
    out.line(&n)?;                                // expect: 8
    out.line(&seen)?;                             // expect: 5

    // Filters of filters, and of maps.
    let odd_squares = (0..10i64).map(|x| x * x).filter(|x| *x % 2 == 1).filter(|x| *x > 1);
    for x in odd_squares {
        out.line(&x)?;
    }
    Result::Ok(())
    // expect: 9
    // expect: 25
    // expect: 49
    // expect: 81
}
