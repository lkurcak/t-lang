// Writing through `l[i]`, `get_mut`, `iter_mut` or `as_mut_slice` lends
// the list past the call, but what's written there is a `T`, which can't
// change how long the list is: those methods say so, and the checker
// keeps `l.len()` while the borrow lives and after (docs/59).
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u64> = List::filled(3, 0);
    l[0] = 3;
    l[1] = 4;
    l[2] += l[1];
    // A reference kept: the length is known after it's done with.
    let r = &mut l[0];
    *r = 5;
    *r += 1;
    l[2] += 1;
    if let Some(x) = l.get_mut(1) {
        *x = 9;
    }
    for x in l.iter_mut() {
        *x += 10;
    }
    let s = l.as_mut_slice();
    s[0] += 100;
    // In a loop: each `l[i]` keeps the length, so the loop's frame does.
    for i in 0..l.len() {
        l[i] = l[i] * 2;
    }
    out.line(&l[0])?; // expect: 232
    out.line(&l[1])?; // expect: 38
    out.line(&l[2])?; // expect: 30
    Ok(())
}
