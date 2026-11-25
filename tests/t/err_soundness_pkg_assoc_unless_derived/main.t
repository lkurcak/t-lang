// An impl's associated effect may be another's, `<W<C> as Go>::e`, whose
// `panic` hangs on `C: Ord`, which `C` derives: so it has none, and the
// method that says it can't panic. Until 0.107.1 the library's checking
// didn't see derived impls there and took it to have `panic`, while this
// program, which sees them, took it to have none: `main` ran a `panic`
// (retrospective 19).
use lib1::{boom, Go};

fn main() with {} {
    let _a = boom().go();
}
