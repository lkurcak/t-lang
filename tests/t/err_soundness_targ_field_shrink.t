// A field whose elements are below its own length: taking a node away
// could leave an edge to it, so the field's refinement is proved again
// after the call it's lent to, and fails.
use alloc::list::List;

#[derive(Discard)]
struct Graph { nodes: List<List<{ t: usize | t < nodes.len() }>> }

impl Graph {
    fn drop_last(&mut self) {
        let _ = self.nodes.pop();
    }
}

fn main() {}
// error: can't prove `t < nodes.len()` for field `nodes` of `Graph`
