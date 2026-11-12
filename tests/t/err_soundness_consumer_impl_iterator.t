// An iterator's impl with a consumer bound on its parameter, which is a
// `&mut`: `next` writes through it.
use alloc::list::List;
use core::mem::replace;

trait Go {
    fn go(self);
}

impl<'a> Go for &'a mut usize {
    fn go(self) {
        *self = 99;
    }
}

#[derive(Discard)]
struct Once<G> {
    g: Option<G>,
}

impl<G: Go + Discard> Iterator for Once<G> {
    type Item = u8;
    effect e = {};
    fn next(&mut self) -> Option<u8> {
        match replace(&mut self.g, None) {
            Some(g) => {
                g.go();
                Some(1)
            }
            None => None,
        }
    }
}

fn drain<I: Iterator + Discard>(mut i: I) with I::e
where
    I::Item: Discard,
{
    let _ = i.next();
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if l.len() > 0 {
        drain(Once { g: Some(&mut l[0]) });
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what `drain` may write behind argument `i`: `I: Iterator` can make or change a value of it
