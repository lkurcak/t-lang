// A derived `Eq` of a type that holds itself compares field by field, all
// the way down, with each field's own `eq`.

#[derive(Discard, Eq)]
enum List<T> { Cons(T, Box<List<T>>), Nil }

#[derive(Discard)]
struct Digit { n: int }

// Only the last digit counts.
impl Eq for Digit {
    fn eq(&self, o: &Digit) -> bool { self.n % 10 == o.n % 10 }
}

#[derive(Discard, Eq)]
struct P<A, B> { a: A, b: Box<B> }

fn same<T: Eq + Discard>(x: T, y: T) -> bool { &&x == &&y }

fn main() with {alloc, panic} {
    let a = List::Cons(Digit { n: 1 }, Box::new(List::Cons(Digit { n: 2 }, Box::new(List::Nil))));
    let b = List::Cons(Digit { n: 11 }, Box::new(List::Cons(Digit { n: 22 }, Box::new(List::Nil))));
    print_bool(a == b); // expect: true
    let c = List::Cons(Digit { n: 11 }, Box::new(List::Nil));
    print_bool(a == c); // expect: false
    print_bool(same(P { a: 1, b: Box::new(Digit { n: 5 }) }, P { a: 1, b: Box::new(Digit { n: 15 }) })); // expect: true
}
