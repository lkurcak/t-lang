// `replace` isn't in the prelude: it's `core::mem::replace`.
fn main() {
    let mut x = 1;
    let y = replace(&mut x, 2);
}
// error: unknown variable `replace`; the core library has `core::mem::replace`: import it with `use core::mem::replace;`
