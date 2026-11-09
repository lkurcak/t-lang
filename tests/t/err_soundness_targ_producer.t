// A trait whose method makes a value: generic code bounded by it can make a
// `T` from nothing, so `T` can't be a refined type.
use alloc::list::List;

trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 9 }
}

fn fill<T: Make>(l: &mut List<T>) with alloc { l.push(T::make()) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    fill(&mut q);
}
// error: can make or change a value of it
