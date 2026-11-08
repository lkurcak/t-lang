// An invariant must hold as the loop starts: else the body would be
// checked from a head that never was.

fn f(n: usize) -> u8 with {} {
    let a: [u8; 3] = [0; 3];
    let mut i: usize = 5;
    while i < n invariant i < 3 {
        a[i];
        i = 0;
    }
    0
}

fn main() {}
// error: can't prove `i < 3` for the loop's invariant, as the loop starts
