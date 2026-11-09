// `l[i].push(x)` is `index_mut`'s, which asks for `i < l.len()`.
use alloc::list::List;

fn main() with alloc {
    let mut l: List<List<u8>> = List::new();
    l.push(List::new());
    l[1].push(1);
}
// error: can't prove `i < self.len()` for argument `i` of `index_mut`
