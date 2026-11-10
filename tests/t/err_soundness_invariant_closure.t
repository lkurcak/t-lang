// A closure that changes a variable the invariant names: what it captured
// `&mut` is a place the checker knows nothing of, so the invariant can't be
// proved even as the loop starts. (Called in the loop, it would hold `i`
// borrowed while the loop reads it, which the borrow checker refuses
// first.)

fn f() -> u8 with {} {
    let a: [u8; 50] = [0; 50];
    let mut i: usize = 0;
    let mut bump = || i = i + 100;
    bump();
    while i < 10 invariant i < 50 {
        a[i];
        i = i + 1;
    }
    0
}

fn main() {}
// error: can't prove `i < 50` for the loop's invariant, as the loop starts
// error: can't prove `i < self.len()` for argument `i` of `index`
// error: can't prove `i < 50` for the loop's invariant, at the end of its body
// error: `bump` may panic, but `f` doesn't say `with panic`
