// A `pub use` can't make a variant seen further than its enum.
mod m {
    #[derive(Discard)]
    enum Hidden {
        A,
    }

    pub use Hidden::A;
// error: `Hidden` is private, so `pub use` can't make its variants public
}

fn main() {}
