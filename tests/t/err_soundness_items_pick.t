// A function generic in an iterator, given two ranges, gives back either:
// the one it gives is within the bounds of one of them, not of both.
fn pick<I: Iterator + Discard>(x: I, y: I, first: bool) -> I {
    match first {
        true => {
            let _ = y;
            x
        },
        false => {
            let _ = x;
            y
        },
    }
}

fn main(os: std::Os) {
    let a: [u8; 8] = [0; 8];
    let mut it = pick(0..8, 0..100, os.args.len() > 0);
    while let Some(r) = it.next() {
        let _ = a[r];
    }
}
// error: can't prove
