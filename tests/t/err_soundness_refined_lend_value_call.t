// A call through a value ensures nothing: a refined place lent to one can't
// be proved again after.
use alloc::list::List;

fn pop(l: &mut List<u8>) {
    let _ = l.pop();
}

fn shrink(n: usize, l: &mut { l: List<u8> | l.len() == n }, f: fn(&mut List<u8>)) {
    f(l);
}

fn main() with alloc {
    let mut l = List::filled(2, 0u8);
    shrink(2, &mut l, pop);
}
// error: can't give `l` to a call through a value
