// Nor through a length: `N`'s value would be needed to know its own type.
const N: usize = make().len();

fn make() -> [u8; N] { [0; N] }
// error: the value of const `N` depends on itself

fn main() {
    let _ = N;
}
