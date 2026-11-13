// `core` already has `impl<I: Iterator> Iterator for &mut I`; a program's
// own impl for a reference to its type may stand beside it only while its
// type isn't an iterator itself.
#[derive(Discard)]
struct Mine {}

impl Iterator for Mine {
    type Item = i64;
    effect e = {};

    fn next(&mut self) -> Option<i64> { None }
}

impl<'a> Iterator for &'a mut Mine { // error: this impl of `Iterator` overlaps `impl Iterator for &'a mut I`
    type Item = i64;
    effect e = {};

    fn next(&mut self) -> Option<i64> { None }
}

fn main() {}
