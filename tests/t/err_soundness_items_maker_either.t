// A function given two iterators of one type can give back either.
fn either<I: DoubleEndedIterator>(x: I, y: I, first: bool) -> core::iter::Rev<I>
where
    I: Discard,
{
    match first {
        true => x.rev(),
        false => y.rev(),
    }
}

fn main(os: std::Os) {
    let a: [u8; 8] = [0; 8];
    for r in either(0..8, 0..100, os.args.len() > 0) {
        let _ = a[r];
    }
}
// error: can't prove
