// A `for`'s invariant holds at the loop's head, before the next value is
// bound: it can't name the pattern's variables.

fn f(n: usize) -> usize {
    let mut s: usize = 0;
    for x in 0..n invariant x < n {
        s = x;
    }
    s
}

fn main() {}
// error: cannot find value `x` in this invariant
