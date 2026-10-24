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

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = shapes::Point { x: 1 };
    out.line(&p.x)?; // expect: 1
    Result::Ok(())
}
