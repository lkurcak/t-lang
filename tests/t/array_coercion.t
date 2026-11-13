// A reference to an array is one to a slice where that's wanted: it's
// given to `as_slice`, which says the slice is as long as the array, so
// `s[2]` needs no check.
fn total(s: &[i64]) -> usize { s.len() }

fn third(s: { s: &[i64] | s.len() == 3 }) -> i64 { s[2] }

fn zero(s: &mut [i64]) {
    for x in s {
        *x = 0;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = [1, 2, 3];
    out.line(&total(&a))?; // expect: 3
    out.line(&third(&a))?; // expect: 3
    let s: &[i64] = &a;
    out.line(&s[1])?; // expect: 2
    let lit: &[u8] = &[7, 8];
    out.line(&lit[1])?; // expect: 8
    zero(&mut a);
    out.line(&a[2]) // expect: 0
}
