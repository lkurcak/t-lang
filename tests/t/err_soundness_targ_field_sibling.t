// Elements below a field before them: making that field smaller is
// checked against them, and fails.
use alloc::list::List;

#[derive(Discard)]
struct Bounded { size: usize, items: List<{ t: usize | t < size }> }

impl Bounded {
    fn shrink(&mut self) {
        if self.size > 0 {
            self.size = self.size - 1;
        }
    }
}

fn main() {}
// error: can't prove `t < size` for field `items` of `Bounded`, which names `size`
