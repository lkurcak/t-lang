// The file of `crate::geo`. It's hermetic: it names other modules from
// `crate::`, `super::` or `self::`.
pub mod area;

/// A point, which only this module can make: `secret` is private.
pub struct Point { pub x: i64, pub y: i64, secret: i64 }

pub fn origin() -> Point with panic { at(0, 0) }

pub fn at(x: i64, y: i64) -> Point with panic { Point { x, y, secret: x + y } }

pub fn close(p: Point) -> i64 {
    let Point { secret, .. } = p;
    secret
}
