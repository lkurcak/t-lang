// `Pattern` is sealed: `str`'s methods count on what its impls find
// being where characters start.
use core::str::{Found, Pattern};

struct Any {}

impl Pattern for Any {
    fn find_in(&self, s: &str) -> Option<Found> { None }
    fn prefix_of(&self, s: &str) -> Option<usize> { None }
    fn suffix_of(&self, s: &str) -> Option<usize> { None }
}

fn main() {}
// error: `Pattern` is sealed: only its own package, `core`, can implement it
