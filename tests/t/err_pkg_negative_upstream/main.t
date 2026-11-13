// What isn't true of another package's type is never taken as known here:
// `q` could make `Thing` an iterator, and then this program would break,
// though `q` broke nothing of its own.
trait Mine {}

impl<T: Iterator> Mine for T {}

impl Mine for q::Thing {} // error: this impl of `Mine` overlaps `impl Mine for T`

fn main() {}
