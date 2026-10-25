// `mem::replace` writes a whole slice through `&mut s`, length and all.
use core::mem;

fn f(a: &[i64]) -> i64 {
    let mut s = a;
    if s.len() > 0 {
        let _old = mem::replace(&mut s, core::slice::empty());
        return s[0];
    }
    0
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
