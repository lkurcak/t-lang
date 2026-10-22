// Type arguments can be written out with `::<..>`, the "turbofish", where
// inference can't work them out or to say them anyway.

#[derive(Discard)]
enum Option<T> { Some(T), None }
#[derive(Discard)]
struct Pair<A, B> { first: A, second: B }
// `T` isn't in either of these signatures, so callers have to say it.
fn none<T>() -> Option<T> { Option::None }
fn count<T>(n: i64) -> i64 { n }
// A type with a parameter none of its fields use: a number tagged with
// what it counts.
#[derive(Discard)]
struct Id<T> { n: i64 }

fn id<T>(x: T) -> T { x }
fn second<A: Discard, B>(_a: A, b: B) -> B { b }
fn apply<T>(f: fn(T) -> T, x: T) -> T { f(x) }
fn is_some<T: Discard>(o: Option<T>) -> bool {
    match o {
        Option::Some(_) => true,
        Option::None => false,
    }
}

fn main() {
    print(id::<i64>(1));                                // expect: 1
    print_bool(is_some(Option::<i64>::None));           // expect: false
    print_bool(is_some(none::<bool>()));                // expect: false
    print(count::<Pair<i64, bool>>(3));                 // expect: 3
    print(apply(id::<i64>, 4));                         // expect: 4
    let p = Pair::<i64, bool> { first: 5, second: true };
    print(p.first);                                     // expect: 5
    let users: Id<Pair<i64, i64>> = Id { n: 6 };
    print(users.n);                                     // expect: 6
    let things = Id::<bool> { n: 7 };
    print(things.n);                                    // expect: 7
    match Option::Some(8) {
        Option::<i64>::Some(n) => print(n),             // expect: 8
        Option::<i64>::None => {}
    }
    let Pair::<i64, bool> { first, .. } = p;
    print(first);                                       // expect: 5
    // `_` stands for a type to infer: write some type arguments, not all.
    print(second::<bool, _>(true, 9i64));                  // expect: 9
    print(unwrap(id::<Option<_>>(Option::Some(10))));   // expect: 10
}

fn unwrap(o: Option<i64>) -> i64 {
    match o {
        Option::Some(n) => n,
        Option::None => 0,
    }
}
