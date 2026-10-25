// `s[i]` is the value at index `i` of a slice, proved in range: `i <
// s.len()` follows from what's known, so it can't panic.
fn sum(s: &[i64]) -> i64 {
    let mut total: i64 = 0;
    let mut i: usize = 0;
    while i < s.len() {
        total = core::num::wrapping_add(total, s[i]);
        i = i + 1;
    }
    total
}

fn first(s: { s: &[i64] | s.len() > 0 }) -> i64 { s[0] }

fn last(s: &[i64]) -> Option<i64> {
    let n = s.len();
    if n == 0 { Option::None } else { Option::Some(s[n - 1]) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(3);
    l.push(4);
    l.push(5);
    let s = l.as_slice();
    out.line(&sum(s))?; // expect: 12
    if !s.is_empty() {
        out.line(&first(s))?; // expect: 3
    }
    out.line(&last(s).unwrap_or(0))?; // expect: 5
    Result::Ok(())
}
