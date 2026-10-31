// `while let` loops as long as the pattern matches; `break` and `continue`
// work as in `while`, labels too.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut it = 0..5;
    let mut sum: i64 = 0;
    while let Option::Some(i) = it.next() {
        if i == 1 {
            continue;
        }
        sum = sum + i;
    }
    out.line(&sum)?; // expect: 9
    let mut chars = "ab  c".chars();
    let mut n: i64 = 0;
    'outer: while let Option::Some(c) = chars.next() {
        let mut k: i64 = 0;
        while let 0..=2 = k {
            k = k + 1;
            if c == ' ' {
                break 'outer;
            }
        }
        n = n + k;
    }
    out.line(&n)?; // expect: 6
    Result::Ok(())
}
