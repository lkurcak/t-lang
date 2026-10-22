// Lifetimes in impls and methods. What a method returns is borrowed from
// `self`, if it takes `&self` and nothing says otherwise, as in Rust.

#[derive(Discard)]
struct Holder<'a> { r: &'a i64 }

impl<'a> Holder<'a> {
    fn new(r: &'a i64) -> Holder<'a> { Holder { r } }

    // Borrowed from what the holder holds, so it outlives the holder.
    fn get(&self) -> &'a i64 { self.r }

    // Borrowed from the holder itself.
    fn peek(&self) -> &i64 { self.r }
}

fn main() with panic {
    let n = 7;
    let r;
    {
        let h = Holder::new(&n);
        r = h.get();
        print(*h.peek());    // expect: 7
    }
    print(*r + 1);           // expect: 8
}
