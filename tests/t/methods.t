// Inherent impls: methods of a type itself. A call `x.m()` goes through any
// references and boxes to what `x` points to, and then takes `&` or `&mut`
// of it if the method takes `&self` or `&mut self`.

use alloc::boxed::Box;

#[derive(Discard)]
struct Counter { n: i64 }

impl Counter {
    /// No `self`: called by its path, `Counter::new()`.
    fn new() -> Counter { Counter { n: 0 } }

    fn get(&self) -> i64 { self.n }

    fn add(&mut self, k: i64) -> &mut Counter with panic {
        self.n += k;
        self
    }

    fn into_inner(self) -> i64 { self.n }

    fn doubled(&self) -> i64 with panic { Self::twice(self.get()) }

    fn twice(x: i64) -> i64 with panic { x * 2 }
}

#[derive(Discard)]
struct Wrapper<T> { x: T }

impl<T> Wrapper<T> {
    fn get(&self) -> &T { &self.x }

    // A method can be generic too.
    fn map<U>(self, f: fn(T) -> U) -> Wrapper<U> { Wrapper { x: f(self.x) } }
}

fn half(x: i64) -> i64 { x / 2 }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut c = Counter::new();
    c.add(1).add(2);
    out.line(&c.get())?;                       // expect: 3
    let r = &mut c;
    r.add(4);
    let rr = &r;
    out.line(&rr.get())?;                      // expect: 7
    out.line(&c.doubled())?;                   // expect: 14
    let b = Box::new(Counter { n: 5 });
    out.line(&b.get())?;                       // expect: 5
    out.line(&c.into_inner())?;                // expect: 7
    let w = Wrapper { x: 10 };
    out.line(&(*w.get()))?;                      // expect: 10
    out.line(&(*w.map(half).get()))?;            // expect: 5
    // A method as a function value.
    let g = Counter::get;
    out.line(&g(&Counter { n: 9 }))?;          // expect: 9
    Result::Ok(())
}
