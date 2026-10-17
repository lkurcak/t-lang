// A `while` can end by its condition, so a `break` to it, label or not,
// can't give it a value.
fn main() {
    let mut i = 0;
    'w: while i < 10 {
        loop {
            if i == 5 { break 'w i; }
            i += 1;
        }
    }
}
// error: a `break` with a value only works in a `loop`
