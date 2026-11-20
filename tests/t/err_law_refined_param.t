// A law only promises: what a call through the method's trait proves is
// all its body may assume (docs/88-iterator-laws.md).
trait A {
    fn f(&self, k: usize) -> usize;
}

trait B: A {
    law fn f(&self, k: { k: usize | k > 0 }) -> usize;
}

fn main() {}
// error: a law can't refine parameter `k`
