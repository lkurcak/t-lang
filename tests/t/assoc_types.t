// Associated types: a trait can say an impl picks a type, as well as
// methods, and code names it with a *projection*: `<T as Tr>::Out`, or
// `T::Out` for short.

trait Container {
    type Item: Discard;

    fn first(&self) -> Self::Item;

    // A default sees `Self::Item` only as the type each impl picks.
    fn first_twice(&self) -> Pair<Self::Item> { Pair { a: self.first(), b: self.first() } }
}

#[derive(Discard)]
struct Pair<T> { a: T, b: T }

#[derive(Discard)]
struct Ints { x: int, y: int }

impl Container for Ints {
    type Item = int;

    fn first(&self) -> int { self.x }
}

#[derive(Discard)]
struct Flags { on: bool }

impl Container for Flags {
    type Item = bool;

    fn first(&self) -> Self::Item { self.on }
}

// Generic code knows nothing of `C::Item` but what the trait says: here,
// that it's `Discard`, so it may be thrown away.
fn ignore_first<C: Container>(c: &C) -> int {
    let _ = c.first();
    1
}

fn get<C: Container>(c: &C) -> <C as Container>::Item { c.first() }

// An equality bound says what the projection is, so it can be used as that.
fn total<C: Container<Item = int>>(c: &C) -> int { c.first() + get(c) }

// A projection's type can be a field's.
#[derive(Discard)]
struct Cache<C: Container> { item: C::Item, from: C }

fn cache<C: Container>(c: C) -> Cache<C> { Cache { item: c.first(), from: c } }

// A trait without the bound, and a bound on the projection where it's used.
trait Make {
    type Out;

    fn make(&self) -> Self::Out;
}

impl Make for Ints {
    type Out = Pair<int>;

    fn make(&self) -> Pair<int> { Pair { a: self.x, b: self.y } }
}

fn make_and_drop<M: Make<Out: Discard>>(m: &M) -> int {
    let _p = m.make();
    2
}

// A supertrait's associated type, by the shorthand too.
trait Sized2: Container {
    fn size(&self) -> int;
}

impl Sized2 for Ints {
    fn size(&self) -> int { 2 }
}

fn first_of<S: Sized2>(s: &S) -> S::Item { s.first() }

fn main() {
    let i = Ints { x: 3, y: 4 };
    print(i.first()); // expect: 3
    let p = i.first_twice();
    print(p.a + p.b);                          // expect: 6
    print_bool((Flags { on: true }).first());  // expect: true
    print(ignore_first(&Flags { on: false })); // expect: 1
    print(total(&i));                          // expect: 6
    let c = cache(Ints { x: 5, y: 6 });
    print(c.item + c.from.y); // expect: 11
    print(make_and_drop(&i)); // expect: 2
    let x: <Ints as Make>::Out = i.make();
    print(x.b);                     // expect: 4
    print(first_of(&i) + i.size()); // expect: 5
}
