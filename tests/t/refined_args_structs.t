// What used to be refusals (docs/53): a struct's type argument, what a
// `Box` holds, and a type parameter an enum has only inside a box can all
// be refined now (docs/60-refined-type-arguments.md). A struct's fields
// are read as the struct's type says they are.
use alloc::boxed::Box;

#[derive(Discard)]
struct Wrap<T> { x: T }

fn unwrap_at(a: [u8; 3], w: Wrap<{ v: usize | v < 3 }>) -> u8 { a[w.x] }

fn boxed() -> Box<{ v: u8 | v < 3 }> with alloc { Box::new(1) }

#[derive(Discard where T: Discard)]
enum Tree<T> { Leaf, Node(Box<Tree<T>>, Box<T>) }

// Each value in the tree indexes the array, at any depth.
fn sum(a: [u8; 3], t: &Tree<{ v: usize | v < 3 }>) -> u64 {
    match t {
        Tree::Leaf => 0,
        Tree::Node(rest, x) => core::num::wrapping_add(a[**x] as u64, sum(a, rest)),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u8; 3] = [1, 10, 100];
    out.line(&unwrap_at(a, Wrap { x: 2 }))?; // expect: 100
    out.line(&*boxed())?; // expect: 1
    let leaf: Tree<{ v: usize | v < 3 }> = Tree::Leaf;
    let t: Tree<{ v: usize | v < 3 }> = Tree::Node(Box::new(Tree::Node(Box::new(leaf), Box::new(0))), Box::new(1));
    out.line(&sum(a, &t))?; // expect: 11
    Ok(())
}
