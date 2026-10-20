// A trait's method that says it may panic may panic, whichever impl runs.
trait Op {
    fn op(&self, n: int) -> int with panic;
}

fn twice<T: Op>(t: &T, n: int) -> int { t.op(t.op(n)) }

#[derive(Discard)]
struct Inc {}

impl Op for Inc {
    fn op(&self, n: int) -> int with panic { n + 1 }
}

fn main() with panic {
    print(twice(&Inc {}, 1));
}
// error: `op` may panic, but `twice` doesn't say `with panic`
