// A `match` that takes a linear value apart drops the parts it doesn't
// bind, there, which runs their `discard`.
use alloc::list::List;

struct W<'a, T> { r: &'a mut T }

impl<'a> Discard for W<'a, usize> {
    fn discard(self) {
        *self.r = 99;
    }
}

enum Two<A, B> { Both(A, B), Neither }

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        let mut k: List<u8> = List::new();
        k.push(1);
        let t = Two::Both(k, W { r: &mut l[0] });
        match t {
            Two::Both(a, _) => {
                let _ = a;
            },
            Two::Neither => {},
        }
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what dropping `t` may write: `W<usize>`'s `discard` can change a value of it
