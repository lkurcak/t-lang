// A loop that lends the list `&mut` may have changed it by the next time
// round: what was known before the loop isn't known at its head.
use alloc::list::List;

fn f(l: &mut List<i64>) -> i64 {
    let mut total: i64 = 0;
    if l.len() > 0 {
        let mut k: i32 = 0;
        while k < 3 {
            total = core::num::wrapping_add(total, l[0]);
            l.pop();
            k = k + 1;
        }
    }
    total
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
