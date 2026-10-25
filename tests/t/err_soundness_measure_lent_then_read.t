// A `&mut` given to a variable changes the list whenever it's used, until
// it's dead: then what the list is is a value nothing's known of.
use alloc::list::List;

fn f(l: List<i64>) -> i64 {
    let mut l = l;
    if l.len() > 0 {
        let r = &mut l;
        r.pop();
        r.pop();
        return l[0];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
