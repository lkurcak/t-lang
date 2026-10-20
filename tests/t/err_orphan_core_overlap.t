// `core` already has `impl<I: Iterator> Iterator for &mut I`; a program's
// own impl for a reference to its type would overlap it, and isn't its to
// write anyway.
#[derive(Discard)]
struct Mine {}

impl Iterator for &mut Mine {
    type Item = int;

    fn next(&mut self) -> Option<int> { Option::None }
}

fn main() {}
// error: `Iterator` and `&mut Mine` are both `core`'s, so only `core` can implement one for the other
