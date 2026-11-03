//! Fresh ids, from one counter for the whole program.

use core::sync::Atomic;

static NEXT: Atomic<u64> = Atomic::new(1);

/// An id no other call gave.
export fn fresh() -> u64 with global { NEXT.fetch_add(1) }

/// A counter of a package's own, which others can use.
export static SEEN: Atomic<u64> = Atomic::new(100);
