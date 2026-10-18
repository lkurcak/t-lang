// Generic functions, structs and enums. Each use picks its own type
// arguments, worked out from the arguments and from where the result goes.

#[derive(Discard)]
enum Option<T> { Some(T), None }
#[derive(Discard)]
struct Pair<A, B> { first: A, second: B }

fn id<T>(x: T) -> T { x }

fn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> {
    Pair { first: p.second, second: p.first }
}

fn map<T, U>(o: Option<T>, f: fn(T) -> U) -> Option<U> {
    match o {
        Option::Some(x) => Option::Some(f(x)),
        Option::None => Option::None,
    }
}

fn unwrap_or<T: Discard>(o: Option<T>, default: T) -> T {
    match o {
        Option::Some(x) => x,
        Option::None => default,
    }
}

// Calls a generic function inside a generic function: `id` at `T`.
fn twice<T>(f: fn(T) -> T, x: T) -> T { f(id(f(x))) }

// Recursion at the same type is fine.
fn repeat<T>(f: fn(T) -> T, x: T, n: int) -> T {
    if n == 0 { x } else { repeat(f, f(x), n - 1) }
}

fn inc(x: int) -> int { x + 1 }

fn main() {
    print(id(1));                                   // expect: 1
    print_bool(id(true));                           // expect: true
    let p = swap(Pair { first: 1, second: false });
    print_bool(p.first);                            // expect: false
    print(p.second);                                // expect: 1
    let n = 10;
    let o = map(Option::Some(4), |x| x * n);
    print(unwrap_or(o, 0));                         // expect: 40
    let none: Option<int> = Option::None;
    print(unwrap_or(map(none, |x| x + 1), -1));     // expect: -1
    print_bool(unwrap_or(map(Option::Some(3), |x| x > 2), false)); // expect: true
    print(twice(inc, 0));                           // expect: 2
    print(repeat(|x| x * 2, 1, 10));                // expect: 1024
    // Generic types nest.
    let nested = Option::Some(Pair { first: Option::Some(1), second: 2 });
    match nested {
        Option::Some(Pair { first: Option::Some(a), second: b }) => print(a + b), // expect: 3
        _ => print(0),
    }
    // A generic function used as a value gets one type, from how it's used.
    print(twice(id, 7));                            // expect: 7
}
