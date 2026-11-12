// An opaque result bounded only by a consumer, whose impl for what it
// stands for writes through it.
use alloc::list::List;

trait Go {
    fn go(self);
}

impl<'a> Go for &'a mut usize {
    fn go(self) {
        *self = 99;
    }
}

fn first<'a>(l: &'a mut List<{ t: usize | t < 5 }>) -> Option<impl Go> {
    if l.len() > 0 { Some(&mut l[0]) } else { None }
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if let Some(g) = first(&mut l) {
        g.go();
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through the result of `first`, `Option<impl Go>`: `Self: Go` can make or change a value of it
