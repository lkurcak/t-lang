// A `loop`'s value is one of what its `break`s give: here 1 or 2, not
// known to be either.

fn f(c: bool) -> u8 with {} {
    let a: [u8; 2] = [0; 2];
    let k: usize = loop {
        if c { break 1; } else { break 2; }
    };
    a[k]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
