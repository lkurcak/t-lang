// And the other way: a list of numbers below 5 can hold a 4, which `f`
// would read as below 4.
use alloc::list::List;

fn f(l: &mut List<{ t: usize | t < 4 }>) -> u8 {
    let a: [u8; 4] = [1, 2, 3, 4];
    match l.pop() {
        Some(i) => a[i],
        None => 0,
    }
}

fn main() with alloc {
    let mut q: List<{ t: usize | t < 5 }> = List::new();
    q.push(4);
    let _ = f(&mut q);
}
// error: can't prove `t < 4` for argument `l` of `f`
