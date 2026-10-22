// The file of `crate::geo::area`.
use super::Point;

pub fn of(p: &Point) -> i64 with panic { p.x * p.y }

pub fn square(n: i64) -> i64 with panic { n * n }
