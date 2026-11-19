// What a law promises names a parameter's value as it was passed, so the
// method it's of can't make that parameter `mut` (H2 of retrospective 15,
// docs/88-iterator-laws.md).
trait Get {
    fn get(&self, k: usize) -> usize;
}

trait Below: Get {
    law fn get(&self, k: usize) -> { n: usize | n <= k };
}

#[derive(Discard)]
struct X {}

impl Get for X {
    fn get(&self, mut k: usize) -> usize {
        k = 100;
        k
    }
}

impl Below for X {}

fn main() {}
// error: `k` can't be `mut` in `get`: what `Below`'s law promises names it, as it was passed; write `let mut k = k;` in the body
