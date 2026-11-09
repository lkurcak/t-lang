// Likewise a value whose type is known only later: `o`'s is known once
// `"abc"` is put in it, after `*id(..)` was checked.
fn id<'a, unsized T>(x: &'a T) -> &'a T { x }

fn main() {
    let mut o = None;
    if false {
        let _y = *id(o.unwrap());
    }
    o = Some("abc");
}
// error: `str` can only be behind a shared reference, as in `&str`
