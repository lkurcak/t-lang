// What the logic knows of a const is what it can tell of its initializer:
// of a call, only what the function's type says, not that it's 3.
fn three() -> usize { 3 }

const N: usize = three();

fn main() {
    let a = [0u8; 3];
    let _ = a[N - 1];
// error: 
}
