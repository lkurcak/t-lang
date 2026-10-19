mod cells {
    #[derive(Eq where T: Discard)]
    pub struct Cell<T> { v: T }
}

fn main() {}
// error: `Cell`'s `Eq` needs `T: Eq`, which its `where` doesn't say
