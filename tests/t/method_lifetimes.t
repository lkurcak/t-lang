// Lifetimes in impls and methods. What a method returns is borrowed from
// `self`, if it takes `&self` and nothing says otherwise, as in Rust.

struct Holder<'a> { r: &'a int }

impl<'a> Holder<'a> {
    fn new(r: &'a int) -> Holder<'a> { Holder { r } }

    // Borrowed from what the holder holds, so it outlives the holder.
    fn get(&self) -> &'a int { self.r }

    // Borrowed from the holder itself.
    fn peek(&self) -> &int { self.r }
}

fn main() {
    let n = 7;
    let r;
    {
        let h = Holder::new(&n);
        r = h.get();
        print(*h.peek());    // expect: 7
    }
    print(*r + 1);           // expect: 8
}
