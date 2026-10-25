// A list's `len()` is a measure, so `l[i]` is proved in range by what's
// known of it, as a slice's is, and `l[i] = v` changes a value. What's
// known of its length after is only what `index_mut` says, which is
// nothing: a slice's length can't change, so writes go through one.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    let mut i: i64 = 0;
    while i < 5 {
        l.push(i * 10);
        i = i + 1;
    }
    let n = l.len();
    if n > 2 {
        l[2] = 7;
    }
    let s = l.as_mut_slice();
    if s.len() > 2 {
        s[1] = 8;
        out.line(&s[2])?; // expect: 7
        out.line(&s[1])?; // expect: 8
    }
    if l.len() == 5 {
        out.line(&l[4])?; // expect: 40
    }
    let mut j: usize = 0;
    let mut total: i64 = 0;
    while j < l.len() {
        total = core::num::wrapping_add(total, l[j]);
        j = j + 1;
    }
    out.line(&total)?; // expect: 85
    Result::Ok(())
}
