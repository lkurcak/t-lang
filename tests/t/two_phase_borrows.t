// `v.push(v.len())`: the `&mut v` for `push` is taken first, as the
// receiver is the first argument, but it's only *reserved* until the call,
// so `v.len()` can still read `v`. That's a two-phase borrow.

#[derive(Discard)]
struct Stack { top: int, size: int }

impl Stack {
    fn len(&self) -> int { self.size }

    fn push(&mut self, x: int) {
        self.top = x;
        self.size += 1;
    }
}

fn main() {
    let mut v = Stack { top: 0, size: 0 };
    v.push(v.len());
    v.push(v.len() + v.top);
    print(v.top);            // expect: 1
    print(v.len());          // expect: 2
    // Through a `&mut`, too.
    let r = &mut v;
    r.push(r.len() * 10);
    print(v.top);            // expect: 20
}
