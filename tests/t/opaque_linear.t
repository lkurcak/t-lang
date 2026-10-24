// What a function returns needn't be `Discard`: then it must be used up,
// here by calling it.

use alloc::boxed::Box;

fn once(b: Box<i64>) -> impl FnOnce() -> i64 { move || *b }

fn main() with alloc {
    let f = once(Box::new(4));
    print(f());                     // expect: 4
    print(once(Box::new(5))());     // expect: 5
}
