// A const's type can be refined: the initializer must meet it.
const N: { n: u8 | n > 5 } = 3;
// error: 

fn main() {
    let _ = N;
}
