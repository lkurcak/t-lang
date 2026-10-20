// A `match` moves out what its arm binds: here the box in `o`.
#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main() with {alloc, panic} {
    let o = Option::Some(Box::new(1));
    match o {
        Option::Some(b) => print(*b),
        Option::None => {}
    }
    // Looking at which variant it is is fine...
    match o {
        Option::Some(_) => print(0),
        Option::None => print(1),
    }
    // ...but not at what was moved.
    match o {
        Option::Some(b) => print(*b),
        Option::None => {}
    }
}
// error: use of moved value `o.0`
