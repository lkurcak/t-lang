// An alias stands only for a type: a literal names the struct itself.
struct Point { x: i64 }
type P = Point;

fn main() {
    let _p = P { x: 1 };
}
// error: `P` is a type alias, so it can only be used as a type; here, write what it stands for: `Point`
