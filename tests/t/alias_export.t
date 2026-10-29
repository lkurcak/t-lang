// An exported alias stands for exported types: other packages can name
// it, and it's part of the package's interface, so it isn't dead code.
export mod geo {
    #[derive(Discard)]
    export struct Point { export x: i64, export y: i64 }
    export type Points<const N: usize> = [Point; N];
    export fn origin() -> Point { Point { x: 0, y: 0 } }
}

export use geo::Points;
export type Total = i64;

fn sum<const N: usize>(ps: &Points<N>) -> Total {
    let mut t = 0;
    for p in ps.iter() {
        t = t + p.x + p.y;
    }
    t
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let ps: Points<2> = [geo::origin(), geo::Point { x: 1, y: 2 }];
    out.line(&sum(&ps))?; // expect: 3
    Result::Ok(())
}
