// A `&mut` kept past where it's made can change the list any time after.
use alloc::list::List;

fn f(l: &mut List<i64>) -> i64 {
    if l.len() > 0 {
        let r = &mut *l;
        r.pop();
        return l[0];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
