struct Point { x: i64 }
type P = Point;

fn get(p: P) -> i64 {
    let P { x } = p;
    x
}

fn main() {}
// error: `P` is a type alias, so it can only be used as a type; here, write what it stands for: `Point`
