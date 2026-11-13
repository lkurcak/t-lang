// Two blanket impls of a trait always overlap, though both bounds are the
// program's own: a type parameter could be anything, so nothing about one
// is known not to hold, and a type could be both `Show` and `Other`.
trait Show {}
trait Other {}
trait Tr {}

impl<T: Show> Tr for T {}

impl<T: Other> Tr for T {} // error: `T` implements `Tr` twice

fn main() {}
