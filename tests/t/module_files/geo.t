// The file of `crate::geo`. It's hermetic: it names other modules from
// `crate::`, `super::` or `self::`.
pub mod area;

/// A point, which only this module can make: `secret` is private.
pub struct Point { pub x: int, pub y: int, secret: int }

pub fn origin() -> Point { at(0, 0) }

pub fn at(x: int, y: int) -> Point { Point { x, y, secret: x + y } }

pub fn close(p: Point) -> int {
    let Point { secret, .. } = p;
    secret
}
