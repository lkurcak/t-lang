// The invariant holds at the head, before the condition runs: what the
// condition changes, the body doesn't get the invariant of.

fn f() -> u8 with {} {
    let a: [u8; 50] = [0; 50];
    let mut i: usize = 0;
    while { i = i + 100; i < 300 } invariant i < 50 {
        a[i];
        i = 0;
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
