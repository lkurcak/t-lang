// What a call leaves behind a `&mut` it's lent is what it ensures, nothing
// more: the list's old length isn't known of it after.
use alloc::list::List;

fn grow(l: &mut List<u8>) with alloc ensures l.len() > 0 {
    l.push(1);
}

fn main() with alloc {
    let mut l: List<u8> = List::new();
    let n = l.len();
    grow(&mut l);
    let m: { m: usize | m == n } = l.len();
    let _ = m;
}
// error: can't prove `m == n` for `m`
