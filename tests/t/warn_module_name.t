mod Geometry {
    pub fn one() -> int { 1 }
}

fn main() {
    print(Geometry::one()); // expect: 1
}
// warning: module `Geometry` should have a snake case name: `geometry`
