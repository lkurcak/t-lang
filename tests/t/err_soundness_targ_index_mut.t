// What's written through `q[0]`, `index_mut`'s `&mut T`, must be a `T`.
use alloc::list::List;

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    q.push(1);
    if q.len() > 0 {
        q[0] = 4;
    }
}
// error: can't prove `t < 4` for what's written
