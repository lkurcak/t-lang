// A closure can make and return another: both borrow `b`, which outlives
// them.
fn main() {
    let b = 2;
    let make = || || b;
    print(make()());                // expect: 2
}
