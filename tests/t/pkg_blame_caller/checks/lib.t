//! A library's own functions can blame their callers, as core's do.

/// Panics unless `n` is even, blaming the caller.
#[blame_caller]
export fn even(n: u8) with panic {
    if n % 2 != 0 { core::panic::panic("odd") }
}

/// Panics unless `n` is even, here: a bug of this library's own.
export fn even_here(n: u8) with panic {
    if n % 2 != 0 { core::panic::panic("odd") }
}
