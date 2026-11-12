// An impl's bound on an associated effect isn't supported yet.
trait Go {
    fn go(self);
}

#[derive(Discard)]
struct Wrap<I> { i: I }

impl<I: Iterator<Item = u64, e = {}> + Discard> Go for Wrap<I> {
    fn go(self) {
        for _ in self.i {}
    }
}

fn main() {}
// error: an impl's bound on an associated effect, `e = {..}`, isn't supported yet
