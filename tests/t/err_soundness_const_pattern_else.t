// The other arm knows only that the value isn't the const's: `i` may be
// past the end.
const THREE: usize = 3;

fn f(a: [u8; 4], i: usize) -> u8 with panic {
    match i {
        THREE => 0,
        _ => a[i],
    }
}

fn main() {
    let _ = f([1, 2, 3, 4], 9);
}
// error: can't prove `i < self.len()`
