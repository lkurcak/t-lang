// Found for a `&S` of any lifetime, so `get` would return a reference to
// a local as `'static`.
struct S { x: int }

trait Get { fn get(self) -> &'static int; }

impl Get for &'static S {
    fn get(self) -> &'static int { &self.x }
}

fn main() {
    let r;
    {
        let s = S { x: 5 };
        r = Get::get(&s);
    }
    print(*r);
}
// error: `&'static S` says a lifetime is `'static`, but an impl of a trait must hold at every lifetime, as finding the impl for a type doesn't look at lifetimes
