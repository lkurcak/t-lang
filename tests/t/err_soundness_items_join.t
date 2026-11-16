// An iterator that's one of two gives what either does.
fn main(os: std::Os) {
    let a: [u8; 8] = [0; 8];
    let it = match os.args.len() > 0 {
        true => (0..100).rev(),
        false => (0..8).rev(),
    };
    for r in it {
        let _ = a[r];
    }
}
// error: can't prove
