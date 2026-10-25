// `s[i] = v`, `s[i] += v` and `&mut s[i]` write through a slice to change,
// with the index proved in range. Changing values doesn't change how many
// there are, so what's known of `s.len()` stays known.
fn bump(s: &mut [i64], i: { i: usize | i < s.len() }) with panic { s[i] += 1; }

fn reverse(s: &mut [i64]) {
    let n = s.len();
    let mut i: usize = 0;
    while i < n / 2 {
        s.swap(i, n - 1 - i);
        i = i + 1;
    }
}

fn zero(r: &mut i64) { *r = 0; }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    let s = l.as_mut_slice();
    if s.len() == 3 {
        s[0] = 10;
        bump(s, 1);
        reverse(s);
        zero(&mut s[2]);
        out.line(&s[0])?; // expect: 3
        out.line(&s[1])?; // expect: 3
        out.line(&s[2])?; // expect: 0
    }
    Result::Ok(())
}
