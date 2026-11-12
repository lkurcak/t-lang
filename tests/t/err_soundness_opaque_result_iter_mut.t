// An opaque result over a refined list's `IterMut`: its bound says it gives
// `&mut usize`s, through which the caller writes what it likes.
use alloc::list::List;

fn it<'a>(l: &'a mut List<{ t: usize | t < 5 }>) -> impl Iterator<Item = &'a mut usize> + Discard {
    l.iter_mut()
}

fn main() {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    for r in it(&mut l) {
        *r = 99;
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through the result of `it`, `impl Iterator<Item = &'a mut usize> + Discard`: `Self::Item = &'a mut usize` can make or change a value of it
