// A type that holds itself with no way to stop has no values: each would
// hold another, without end.

struct Stream { head: int, next: Box<Stream> }

// Each holding the other is the same.
struct Ping { pong: Box<Pong> }
struct Pong { ping: Box<Ping> }

// Every variant holds one.
enum Tree { Leaf(Box<Tree>), Node(Box<Tree>, Box<Tree>) }

// Generic or not.
struct Chain<T> { value: T, next: Box<Chain<T>> }

// Fine: `Nil` stops it.
enum List { Nil, Cons(int, Box<List>) }

// Fine: no values on purpose.
enum Never {}

// Fine: no values, but only since `Stream` has none, and that's warned
// about already.
struct Holder { s: Stream }

#[allow(infinite_types)]
struct Quiet { next: Box<Quiet> }

fn main() {
    let l = List::Cons(1, Box::new(List::Nil));
    match l {
        List::Nil => print(0),
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
