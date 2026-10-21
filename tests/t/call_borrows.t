// A call works out its arguments, then takes its borrows of places, just
// before the call: `v.push(v.len())` is `let n = v.len(); v.push(n)`.

#[derive(Discard)]
struct Stack { top: int, size: int }

impl Stack {
    fn len(&self) -> int { self.size }

    fn push(&mut self, x: int) with panic {
        self.top = x;
        self.size += 1;
    }

    fn clear(&mut self) { self.size = 0 }
}

fn push(s: &mut Stack, x: int) with panic { s.push(x) }

#[derive(Discard)]
struct Holder { b: Box<Stack> }

fn make() -> Stack { Stack { top: 0, size: 7 } }

fn main() with {panic, alloc} {
    let mut v = Stack { top: 0, size: 0 };
    v.push(v.len());
    v.push(v.len() + v.top);
    print(v.top);            // expect: 1
    print(v.len());          // expect: 2
    // Through a `&mut`, too.
    let r = &mut v;
    r.push(r.len() * 10);
    print(v.top);            // expect: 20
    // A `&mut` written as an argument waits too.
    push(&mut v, v.len());
    print(v.top);            // expect: 3
    // The arguments may even change what's borrowed.
    v.push({
        v.clear();
        v.size = 5;
        1
    });
    print(v.len());          // expect: 6
    // The box is borrowed after it's replaced, so it's the new one.
    let mut h = Holder { b: Box::new(Stack { top: 0, size: 0 }) };
    h.b.push({
        h.b = Box::new(Stack { top: 0, size: 10 });
        4
    });
    print(h.b.size);         // expect: 11
    // What isn't a place is worked out in order.
    print(make().len());     // expect: 7
}
