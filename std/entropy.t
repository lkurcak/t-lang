//! Entropy: bytes no one can guess, from the operating system, to seed a
//! random number generator with, or to key a hash map (see
//! docs/49-time-entropy-terminal.md). It's in the `Os` that `main` is
//! given, as `os.entropy`: a function handed none does the same whenever
//! it's called.

use crate::io::Error;

/// Where unguessable bytes come from: what `fn main(os: Os)` is given in
/// its `os.entropy`. Taking some leaves none the less for anyone else, so
/// it's `Copy`.
#[derive(Discard, Copy, Send, Sync)]
#[intrinsic]
export struct Entropy {}

impl Entropy {
    /// Fills `buf` with bytes no one can guess, as good as the operating
    /// system has for keys. It can fail only where there are none to be
    /// had, as `Unsupported`.
    export fn fill(&self, buf: &mut [u8]) -> Result<(), Error> {
        crate::io::status(self.os_fill(buf))
    }

    /// A number no one can guess: eight bytes of `fill`.
    export fn u64(&self) -> Result<u64, Error> {
        let mut b = [0u8; 8];
        self.fill(b.as_mut_slice())?;
        let mut n = 0u64;
        for x in b.as_slice().iter() {
            n = core::num::wrapping_mul(n, 256) | core::num::wrap::<u8, u64>(*x);
        }
        Ok(n)
    }

    /// Fills `buf`: 0, or a failure, as `std::io::status` reads it.
    #[intrinsic]
    fn os_fill(&self, buf: &mut [u8]) -> u64;
}
