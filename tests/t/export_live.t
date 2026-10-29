// What's exported is used by other packages, so it isn't dead code even
// where nothing in its own package uses it. A `pub` function nothing uses
// is, as its package is all that can.
export fn for_others() -> i64 { 1 }

pub fn for_no_one() -> i64 { 2 }

fn main() {}
// warning: function `for_no_one` is never used
