// Inherent impls: methods of a type itself. A call `x.m()` goes through any
// references and boxes to what `x` points to, and then takes `&` or `&mut`
// of it if the method takes `&self` or `&mut self`.

struct Counter { n: int }

impl Counter {
    /// No `self`: called by its path, `Counter::new()`.
    fn new() -> Counter { Counter { n: 0 } }

    fn get(&self) -> int { self.n }

    fn add(&mut self, k: int) -> &mut Counter {
        self.n += k;
        self
    }

    fn into_inner(self) -> int { self.n }

    fn doubled(&self) -> int { Self::twice(self.get()) }

    fn twice(x: int) -> int { x * 2 }
}

struct Wrapper<T> { x: T }

impl<T> Wrapper<T> {
    fn get(&self) -> &T { &self.x }

    // A method can be generic too.
    fn map<U>(self, f: fn(T) -> U) -> Wrapper<U> { Wrapper { x: f(self.x) } }
}

fn half(x: int) -> int { x / 2 }

fn main() {
    let mut c = Counter::new();
    c.add(1).add(2);
    print(c.get());                       // expect: 3
    let r = &mut c;
    r.add(4);
    let rr = &r;
    print(rr.get());                      // expect: 7
    print(c.doubled());                   // expect: 14
    let b = Box::new(Counter { n: 5 });
    print(b.get());                       // expect: 5
    print(c.into_inner());                // expect: 7
    let w = Wrapper { x: 10 };
    print(*w.get());                      // expect: 10
    print(*w.map(half).get());            // expect: 5
    // A method as a function value.
    let g = Counter::get;
    print(g(&Counter { n: 9 }));          // expect: 9
}
