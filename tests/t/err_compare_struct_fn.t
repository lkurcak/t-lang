struct Handler { f: fn(int) -> int }
fn id(x: int) -> int { x }
fn main() -> bool {
    Handler { f: id } == Handler { f: id }
}
// error: cannot compare `Handler` with `==`: it contains a function
