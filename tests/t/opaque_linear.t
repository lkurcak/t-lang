// What a function returns needn't be `Discard`: then it must be used up,
// here by calling it.
fn once(b: Box<int>) -> impl FnOnce() -> int { move || *b }

fn main() with {alloc, panic} {
    let f = once(Box::new(4));
    print(f());                     // expect: 4
    print(once(Box::new(5))());     // expect: 5
}
