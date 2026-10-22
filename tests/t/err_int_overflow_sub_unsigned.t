fn sub(a: usize, b: usize) -> usize with panic { a - b }

fn main() with panic {
    print(sub(1, 2));
}
// error: runtime error: integer overflow
