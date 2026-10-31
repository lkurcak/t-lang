// A result's refinement can name what's behind a `&mut` parameter: it's
// about what that was at the call. `as_mut_slice` says how long the slice
// is: as long as the list was, which a slice to change can't change.
use alloc::list::List;

fn take_last(l: &mut List<i64>) -> { r: usize | r == old(l.len()) } {
    let n = l.len();
    l.pop();
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(5);
    l.push(6);
    let n = l.len();
    let s = l.as_mut_slice();
    if n == 2 {
        s[0] = s[1];
        out.line(&s[0])?; // expect: 6
    }
    out.line(&take_last(&mut l))?; // expect: 2
    let t: &[i64] = &*l.as_slice();
    if !t.is_empty() {
        out.line(&t[0])?; // expect: 6
    }
    Result::Ok(())
}
