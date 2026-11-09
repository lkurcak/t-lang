// A struct result whose fields include a `&mut usize` into `self`: the
// caller can change the length through it, so the method can't promise
// the length stays.
struct Counted<T> {
    n: usize,
    v: T,
}

struct Peek<'a, T> {
    item: &'a mut T,
    count: &'a mut usize,
}

impl<T> Counted<T> {
    #[measure]
    fn count(&self) -> usize { self.n }

    fn peek(&mut self) -> Peek<T> ensures self.count() == old(self.count()) {
        Peek { item: &mut self.v, count: &mut self.n }
    }
}

fn main() {
    let mut c = Counted { n: 1, v: 0u8 };
    if c.count() == 1 {
        let p = c.peek();
        *p.count = 5;
        *p.item = 1;
        let n: { n: usize | n == 1 } = c.count();
        let _ = n;
    }
}
// error: `ensures` can't name `self` at return
