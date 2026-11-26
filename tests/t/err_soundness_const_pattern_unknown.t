// Of a const worked out by a call, the logic knows what the function's
// type says, here nothing: so `i == K` doesn't put `i` in bounds.
fn k() -> usize {
    7
}

const K: usize = k();

fn f(a: [u8; 4], i: usize) -> u8 with panic {
    match i {
        K => a[i],
        _ => 0,
    }
}

fn main() {
    let _ = f([1, 2, 3, 4], 7);
}
// error: can't prove `i < self.len()`
