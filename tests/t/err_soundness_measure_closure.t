// A closure that captures a list by `&mut` can change it whenever it's
// called.
use alloc::list::List;

fn f(l: List<i64>) -> i64 {
    let mut l = l;
    if l.len() > 0 {
        let mut shrink = || { l.pop(); };
        shrink();
        return l[0];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
