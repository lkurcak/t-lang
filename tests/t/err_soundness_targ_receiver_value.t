// A method given a `&mut` to a field of what a call gave (here `nodes[0]`,
// `*nodes.index_mut(0)`): what it may write must keep what's said of the
// field, though it's no variable's place.
use alloc::list::List;

#[derive(Discard)]
struct Node { out: List<{ t: usize | t < 4 }> }

fn main() with alloc {
    let mut nodes: List<Node> = List::new();
    nodes.push(Node { out: List::new() });
    if nodes.len() > 0 {
        nodes[0].out.push(9);
    }
}
// error: can't prove `t < 4`
