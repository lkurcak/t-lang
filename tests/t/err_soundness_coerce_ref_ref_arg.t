// The same for `&rb`, a `&&Box<i64>`.

use alloc::boxed::Box;

fn get(r: &i64) -> &i64 { r }

fn main() {
    let b = Box::new(1);
    let rb = &b;
    let r = get(&rb);
    print(*r);
}
// error: expected `i64`, found `&Box<{integer}>`
