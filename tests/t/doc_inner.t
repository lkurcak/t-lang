//! `//!` lines document the module they start: here, the crate root.
//! They come before anything else in it.

// A plain comment can come between.
mod shapes {
    //! Shapes, and an empty module's docs.

    mod none {
        //! Nothing here.
    }

    /// A point.
    #[derive(Discard)]
    pub struct Point { pub x: i64 }
}

fn main() {
    let p = shapes::Point { x: 1 };
    print(p.x); // expect: 1
}
