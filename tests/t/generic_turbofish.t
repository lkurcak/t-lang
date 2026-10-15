// Type arguments can be written out with `::<..>`, the "turbofish", where
// inference can't work them out or to say them anyway.

enum Option<T> { Some(T), None }
struct Pair<A, B> { first: A, second: B }
// `T` isn't in either of these signatures, so callers have to say it.
fn none<T>() -> Option<T> { Option::None }
fn count<T>(n: int) -> int { n }
// A type with a parameter none of its fields use: a number tagged with
// what it counts.
struct Id<T> { n: int }

fn id<T>(x: T) -> T { x }
fn second<A, B>(_a: A, b: B) -> B { b }
fn apply<T>(f: fn(T) -> T, x: T) -> T { f(x) }
fn is_some<T>(o: Option<T>) -> bool {
    match o {
        Option::Some(_) => true,
        Option::None => false,
    }
}

fn main() {
    print(id::<int>(1));                                // expect: 1
    print_bool(is_some(Option::<int>::None));           // expect: false
    print_bool(is_some(none::<bool>()));                // expect: false
    print(count::<Pair<int, bool>>(3));                 // expect: 3
    print(apply(id::<int>, 4));                         // expect: 4
    let p = Pair::<int, bool> { first: 5, second: true };
    print(p.first);                                     // expect: 5
    let users: Id<Pair<int, int>> = Id { n: 6 };
    print(users.n);                                     // expect: 6
    let things = Id::<bool> { n: 7 };
    print(things.n);                                    // expect: 7
    match Option::Some(8) {
        Option::<int>::Some(n) => print(n),             // expect: 8
        Option::<int>::None => {}
    }
    let Pair::<int, bool> { first, .. } = p;
    print(first);                                       // expect: 5
    // `_` stands for a type to infer: write some type arguments, not all.
    print(second::<bool, _>(true, 9));                  // expect: 9
    print(unwrap(id::<Option<_>>(Option::Some(10))));   // expect: 10
}

fn unwrap(o: Option<int>) -> int {
    match o {
        Option::Some(n) => n,
        Option::None => 0,
    }
}
