// A `struct` or `enum` in a block is local to it: it means what it would
// at the top level, and its name is in scope in the whole block.
#[derive(Discard)]
struct P { x: int }

fn total() -> int with panic {
    // Used before it's declared.
    let pair = Pair { left: P { x: 1 }, right: P { x: 2 } };
    /// Two of the top-level `P`.
    #[derive(Discard)]
    struct Pair { left: P, right: P }
    // A nested `fn` can use it, in its signature too.
    fn sum(p: Pair) -> int with panic { p.left.x + p.right.x }
    sum(pair)
}

fn shapes() -> int with panic {
    // Generic.
    #[derive(Discard)]
    enum Shape<T> { Circle(T), Rect(T, T), Empty }
    fn area(s: Shape<int>) -> int with panic {
        match s {
            Shape::Circle(r) => 3 * r * r,
            Shape::Rect(w, h) => w * h,
            Shape::Empty => 0,
        }
    }
    area(Shape::Circle(2)) + area(Shape::Rect(2, 3)) + area(Shape::<int>::Empty)
}

fn main() with panic {
    print(total());  // expect: 3
    print(shapes()); // expect: 18
    // Shadows the top-level `P`.
    #[derive(Discard)]
    struct P { y: bool }
    print_bool(P { y: true }.y); // expect: true
    // A value of a local type can leave its block: nothing outside can
    // name the type, but inference doesn't need to.
    let q = {
        #[derive(Discard)]
        struct Q { n: int }
        Q { n: 7 }
    };
    print(q.n); // expect: 7
    // A second `P` in `main`, in a block of its own.
    {
        #[derive(Discard)]
        struct P { z: int }
        let P { z } = P { z: 9 };
        print(z); // expect: 9
    }
    print(other()); // expect: 4
    print(outer()); // expect: 5
}

// Another function's `P`.
fn other() -> int {
    #[derive(Discard)]
    struct P { y: int }
    P { y: 4 }.y
}

// In a nested `fn`.
fn outer() -> int with panic {
    fn inner() -> int with panic {
        #[derive(Discard)]
        enum Sign { Minus, Plus }
        fn value(s: Sign) -> int {
            match s {
                Sign::Minus => -5,
                Sign::Plus => 5,
            }
        }
        value(Sign::Plus) + value(Sign::Minus) + value(Sign::Plus)
    }
    inner()
}
