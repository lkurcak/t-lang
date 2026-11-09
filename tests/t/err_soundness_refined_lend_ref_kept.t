// What a call needed of what an unrefined `&mut` points to held then, not
// after: `push` changes the length, so it isn't `n` any more.
use alloc::list::List;

fn first(n: usize, l: &mut { l: List<u8> | l.len() == n }) {
    if n > 0 {
        l.set(0, 1);
    }
}

fn grow(l: &mut List<u8>) -> usize with alloc {
    let n = l.len();
    first(n, l);
    l.push(1);
    let k: { k: usize | k == n } = l.len();
    k
}

fn main() with alloc {
    let mut l = List::filled(2, 0u8);
    let _ = grow(&mut l);
}
// error: can't prove `k == n` for `k`
