// `Int` makes values: a literal is one of any integer type.
use alloc::list::List;

fn nine<T: Int>(l: &mut List<T>) with alloc { l.push(9) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    nine(&mut q);
}
// error: can make or change a value of it
