// An invariant about one variable keeps nothing of another the loop
// changes: `j` is forgotten at the head, as without one.

fn f(n: usize) -> u8 with {} {
    let a: [u8; 2] = [0; 2];
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < n invariant i <= n {
        a[j];
        j = j + 1;
        i = i + 1;
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
