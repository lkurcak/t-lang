// A `while` left by a `break`, not by its condition: neither the invariant
// nor the condition's being false holds after it, only how things were at
// the `break`.

fn f(n: usize) -> u8 with {} {
    let a: [u8; 4] = [0; 4];
    let mut i: usize = 0;
    while i < 3 invariant i <= 3 {
        i = i + 1;
        if n > 7 {
            i = 9;
            break;
        }
    }
    a[i]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
