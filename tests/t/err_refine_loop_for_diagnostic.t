// A `for` checks every iteration: `d` starts positive, but a previous
// iteration may have assigned zero. Explain the original source loop.
fn positive(d: { v: i64 | v > 0 }) { let _ = d; }

fn main() {
    let mut d = 1;
    for _step in 0..2i64 {
        positive(d);
        d = 0;
    }
}
// error: assignment in this loop may change `d`
