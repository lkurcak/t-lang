// An opaque type inside the result, an `Option`'s.
use alloc::list::List;

fn it<'a>(
    l: &'a mut List<{ t: usize | t < 5 }>,
) -> Option<impl Iterator<Item = &'a mut usize> + Discard> { Some(l.iter_mut()) }

fn main() {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    if let Some(i) = it(&mut l) {
        for r in i {
            *r = 99;
        }
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through the result of `it`
