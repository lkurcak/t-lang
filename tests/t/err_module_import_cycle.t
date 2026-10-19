mod a {
    pub use super::b::x;
}

mod b {
    pub use super::a::x;
}

fn main() { print(a::x()); }
// error: these imports go round in a circle
