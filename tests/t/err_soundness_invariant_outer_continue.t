// `continue 'outer` from an inner loop goes round the outer one: its
// invariant must hold there.

fn f(c: bool) -> u8 with {} {
    let mut i: usize = 0;
    'outer: while i < 3 invariant i < 4 {
        loop {
            i = 9;
            if c { continue 'outer; }
            i = 0;
            break;
        }
        i = i + 1;
    }
    0
}

fn main() {}
// error: can't prove `i < 4` for the loop's invariant, at this `continue`
