// A closure that changes a variable the invariant names, called in the
// loop: what it captured `&mut` is a place the checker knows nothing of,
// so the invariant can't be proved even as the loop starts.

fn f() -> u8 with {} {
    let a: [u8; 50] = [0; 50];
    let mut i: usize = 0;
    let mut bump = || i = i + 100;
    while i < 10 invariant i < 50 {
        bump();
        a[i];
    }
    0
}

fn main() {}
// error: can't prove `i < 50` for the loop's invariant, as the loop starts
