// An impl's method can't do more than the trait's says: `total` calls
// `Half::run` knowing only the trait, which says it can't panic.
trait Run {
    fn run(&self, n: int) -> int;
}

#[derive(Discard)]
struct Half {}

impl Run for Half {
    fn run(&self, n: int) -> int with panic { n / 2 }
}

fn total<R: Run>(r: &R) -> int { r.run(4) }

fn main() {
    print(total(&Half {}));
}
// error: `run` may panic, but `Run`'s `run` can't, so its impls can't either
