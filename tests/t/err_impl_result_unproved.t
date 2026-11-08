trait Size {
    fn size(&self) -> usize;
}

struct Three {}

impl Size for Three {
    fn size(&self) -> { n: usize | n == 3 } { 4 }
}

fn main() {}
// error: can't prove `n == 3` for the result of `size`
