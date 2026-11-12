// A list's `discard` drops its elements, each by its own.
use alloc::list::List;

struct W<'a, T> { r: &'a mut T }

impl<'a> Discard for W<'a, usize> {
    fn discard(self) {
        *self.r = 99;
    }
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        let mut ws: List<W<{ t: usize | t < 5 }>> = List::new();
        ws.push(W { r: &mut l[0] });
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what dropping `ws` may write: `W<usize>`'s `discard` can change a value of it
