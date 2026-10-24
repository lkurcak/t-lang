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
struct Ints { x: i64, y: i64 }

impl Container for Ints {
    type Item = i64;

    fn first(&self) -> i64 { self.x }
}

#[derive(Discard)]
struct Flags { on: bool }

impl Container for Flags {
    type Item = bool;

    fn first(&self) -> Self::Item { self.on }
}

// Generic code knows nothing of `C::Item` but what the trait says: here,
// that it's `Discard`, so it may be thrown away.
fn ignore_first<C: Container>(c: &C) -> i64 {
    let _ = c.first();
    1
}

fn get<C: Container>(c: &C) -> <C as Container>::Item { c.first() }

// An equality bound says what the projection is, so it can be used as that.
fn total<C: Container<Item = i64>>(c: &C) -> i64 with panic { c.first() + get(c) }

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
    type Out = Pair<i64>;

    fn make(&self) -> Pair<i64> { Pair { a: self.x, b: self.y } }
}

fn make_and_drop<M: Make<Out: Discard>>(m: &M) -> i64 {
    let _p = m.make();
    2
}

// A supertrait's associated type, by the shorthand too.
trait Sized2: Container {
    fn size(&self) -> i64;
}

impl Sized2 for Ints {
    fn size(&self) -> i64 { 2 }
}

fn first_of<S: Sized2>(s: &S) -> S::Item { s.first() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let i = Ints { x: 3, y: 4 };
    out.line(&i.first())?; // expect: 3
    let p = i.first_twice();
    out.line(&(p.a + p.b))?;                          // expect: 6
    out.line(&(Flags { on: true }).first())?;  // expect: true
    out.line(&ignore_first(&Flags { on: false }))?; // expect: 1
    out.line(&total(&i))?;                          // expect: 6
    let c = cache(Ints { x: 5, y: 6 });
    out.line(&(c.item + c.from.y))?; // expect: 11
    out.line(&make_and_drop(&i))?; // expect: 2
    let x: <Ints as Make>::Out = i.make();
    out.line(&x.b)?;                     // expect: 4
    out.line(&(first_of(&i) + i.size()))?; // expect: 5
    Result::Ok(())
}
