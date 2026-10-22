// An impl's bound with no `with` holds only of what can't panic, so `go`,
// which knows only that `W<F>` implements `Run`, needn't say it may.
trait Run {
    fn run(&self) -> i64;
}

#[derive(Discard where F: Discard)]
struct W<F> { f: F }

impl<F: Fn() -> i64> Run for W<F> {
    fn run(&self) -> i64 { (self.f)() }
}

fn go<R: Run>(r: &R) -> i64 { r.run() }

fn main() {
    let z: i64 = 0;
    let w = W { f: || 1 / z };
    print(go(&w));
}
// error: `go` needs its `R` to implement `Run`, and `W<{closure}>` doesn't
