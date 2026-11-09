// Nor one whose method changes the value behind its `&mut self`.
use alloc::list::List;

trait Bump {
    fn bump(&mut self);
}

impl Bump for usize {
    fn bump(&mut self) { *self = 99 }
}

fn bump_first<T: Bump>(l: &mut List<T>) {
    if l.len() > 0 {
        l[0].bump();
    }
}

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    q.push(1);
    bump_first(&mut q);
}
// error: can make or change a value of it
