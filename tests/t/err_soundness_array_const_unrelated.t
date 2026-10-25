// What's known of one array's length says nothing of another's.
fn at<const N: usize, const M: usize>(a: &[i64; N], b: &[i64; M]) -> i64 {
    if a.len() > 0 { b[0] } else { 0 }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
