// A field's type argument can be refined, naming the fields before it and
// the field itself: a graph whose every edge goes to one of its nodes
// (docs/60-refined-type-arguments.md). When what it names changes, it's
// proved again: after a node is added, every target below the old count
// is below the new one.
use alloc::list::List;
use core::num::wrapping_add;

#[derive(Discard)]
struct Node<I> { out: List<I> }

#[derive(Discard)]
struct Graph { nodes: List<Node<{ t: usize | t < nodes.len() }>> }

impl Graph {
    fn new() -> Graph { Graph { nodes: List::new() } }

    #[measure]
    fn count(&self) -> usize { self.nodes.len() }

    fn add_node(&mut self) -> { id: usize | id < self.count() }
    with alloc
    ensures
        self.count() == old(self.count()) + 1,
    {
        let id = self.nodes.len();
        self.nodes.push(Node { out: List::new() });
        id
    }

    fn add_edge(&mut self, from: { v: usize | v < self.count() }, to: { v: usize | v < self.count() })
    with alloc
    ensures
        self.count() == old(self.count()),
    {
        self.nodes[from].out.push(to);
    }

    // How many nodes `start` reaches: no target is checked, as each is a
    // node.
    fn reached(&self, start: { v: usize | v < self.count() }) -> usize with alloc {
        let mut seen = List::filled(self.nodes.len(), false);
        seen.set(start, true);
        let mut todo: List<{ t: usize | t < self.count() }> = List::new();
        todo.push(start);
        let mut n: usize = 0;
        while let Some(v) = todo.pop() {
            n = wrapping_add(n, 1);
            for t in self.nodes[v].out.iter() {
                if !seen[*t] {
                    seen.set(*t, true);
                    todo.push(*t);
                }
            }
        }
        n
    }
}

// A sibling field: the targets are below `size`, which only grows.
#[derive(Discard)]
struct Bounded { size: usize, items: List<{ t: usize | t < size }> }

impl Bounded {
    fn grow(&mut self) {
        if self.size < 100 {
            self.size = self.size + 1;
        }
    }

    fn add_last(&mut self) with alloc {
        if self.size > 0 {
            self.items.push(self.size - 1);
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut g = Graph::new();
    let a = g.add_node();
    let b = g.add_node();
    let c = g.add_node();
    let d = g.add_node();
    g.add_edge(a, b);
    g.add_edge(b, c);
    g.add_edge(c, a);
    let _ = d;
    out.line(&g.reached(a))?; // expect: 3
    out.line(&g.reached(d))?; // expect: 1
    let mut s = Bounded { size: 0, items: List::new() };
    s.grow();
    s.add_last();
    s.grow();
    s.add_last();
    out.line(&s.items.len())?; // expect: 2
    Ok(())
}
