// `continue` goes round again, to where the box was moved last time.

use alloc::boxed::Box;

fn consume(b: Box<i64>) -> i64 { *b }
fn main() with alloc {
    let b = Box::new(1);
    let mut i: i64 = 0;
    while i < 3 {
        i += 1;
        print(consume(b));
        continue;
    }
}
// error: use of moved value `b`
