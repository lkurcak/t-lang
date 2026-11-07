// `T` is in a field of `Node` only inside a `Box`, which a refinement of
// it couldn't reach.
use alloc::boxed::Box;

enum Tree<T> { Leaf, Node(Box<Tree<T>>, Box<T>) }

fn f(t: Tree<{ v: u8 | v < 3 }>) {}

fn main() {}
// error: `T` is in a field of variant `Node` of `Tree` other than as its whole type
