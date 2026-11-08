// A `break` out of an outer loop from inside an inner one is one of the
// outer loop's exits, as things are there.

fn f(c: bool) -> u8 with {} {
    let a: [u8; 4] = [0; 4];
    let mut i: usize = 0;
    'outer: loop invariant i < 4 {
        loop {
            i = i + 4;
            if c { break 'outer; }
            i = 0;
            break;
        }
    }
    a[i]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
