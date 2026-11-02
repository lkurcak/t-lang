//! Sizes, worked out when this package is compiled.

/// A block's size in bytes.
export const BLOCK: usize = 8;

fn twice(n: usize) -> usize with panic { n * 2 }

/// Two blocks.
export const PAIR: usize = twice(BLOCK);

/// A block of bytes.
export struct Block { bytes: [u8; BLOCK] }

impl Block {
    export fn zeros() -> Block { Block { bytes: [0; BLOCK] } }
    export fn last(&self) -> u8 { self.bytes[BLOCK - 1] }
}
