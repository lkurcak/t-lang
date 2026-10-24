// A type that holds itself with no way to stop has no values: each would
// hold another, without end.

#[derive(Discard)]
struct Stream { head: i64, next: Box<Stream> }

// Each holding the other is the same.
#[derive(Discard)]
struct Ping { pong: Box<Pong> }
#[derive(Discard)]
struct Pong { ping: Box<Ping> }

// Every variant holds one.
#[derive(Discard)]
enum Tree { Leaf(Box<Tree>), Node(Box<Tree>, Box<Tree>) }

// Generic or not.
#[derive(Discard)]
struct Chain<T> { value: T, next: Box<Chain<T>> }

// Fine: `Nil` stops it.
#[derive(Discard)]
enum List { Nil, Cons(i64, Box<List>) }

// Fine: no values on purpose.
#[derive(Discard)]
enum Never {}

// Fine: no values, but only since `Stream` has none, and that's warned
// about already.
#[derive(Discard)]
struct Holder { s: Stream }

#[allow(infinite_types)]
#[derive(Discard)]
struct Quiet { next: Box<Quiet> }

fn main() with alloc {
    let l = List::Cons(1, Box::new(List::Nil));
    match l {
        List::Nil => print(0i64),
        List::Cons(x, _) => print(x), // expect: 1
    }
}
// warning: struct `Stream` has no values: each would hold another `Stream`, without end; it needs a way to stop, like a variant that doesn't hold one
// warning: struct `Ping` has no values: each would hold another `Ping`, without end; it needs a way to stop, like a variant that doesn't hold one
// warning: struct `Pong` has no values: each would hold another `Pong`, without end; it needs a way to stop, like a variant that doesn't hold one
// warning: enum `Tree` has no values: each would hold another `Tree`, without end; it needs a way to stop, like a variant that doesn't hold one
// warning: struct `Chain` has no values: each would hold another `Chain`, without end; it needs a way to stop, like a variant that doesn't hold one
// warning: enum `Never` is never used
// warning: struct `Holder` is never constructed
