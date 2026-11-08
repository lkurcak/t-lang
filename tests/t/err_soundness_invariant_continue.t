// A `continue` goes round again: the invariant must hold there.

fn f(c: bool) -> u8 with {} {
    let a: [u8; 4] = [0; 4];
    let mut i: usize = 0;
    while i < 3 invariant i < 4 {
        if c {
            i = 7;
            continue;
        }
        a[i];
        i = i + 1;
    }
    0
}

fn main() {}
// error: can't prove `i < 4` for the loop's invariant, at this `continue`
