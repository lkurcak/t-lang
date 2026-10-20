// The file of `crate::geo::area`.
use super::Point;

pub fn of(p: &Point) -> int with panic { p.x * p.y }

pub fn square(n: int) -> int with panic { n * n }
