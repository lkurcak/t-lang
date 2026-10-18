// A local type nothing reachable constructs is dead, like a top-level one.
// One in a function that's never used goes without saying.
fn main() {
    #[derive(Discard)]
    struct Unused { x: int }
    #[derive(Discard)]
    enum E { A, B }
    let _e = E::A;
    #[derive(Discard)]
    struct _OnPurpose {}
    print(1); // expect: 1
}

fn dead() {
    #[derive(Discard)]
    struct InDead {}
}
// warning: struct `Unused` is never constructed
// warning: variant `E::B` is never constructed
// warning: function `dead` is never used
