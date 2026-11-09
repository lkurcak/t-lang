// `&mut D` is fine, but no `&mut str` is ever made for it to be: there's
// no such type to borrow a `str` as.
fn again<'a, unsized T>(x: &'a mut T) -> &'a mut T { x }

fn main() {
    let s: &str = "abc";
    let _m = again(&mut *s);
}
// error: cannot borrow `*s` as mutable
