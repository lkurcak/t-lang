// Every box is freed exactly once, wherever it ends up: the interpreter
// checks, and fails on a double free, a use after a move, or a box that's
// never freed.

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

#[derive(Discard)]
struct Two { a: Box<int>, b: Box<int> }

fn two(n: int) -> Two with {alloc, panic} { Two { a: Box::new(n), b: Box::new(n + 1) } }

fn first(t: Two) -> int { *t.a }    // `t.b` is freed when `first` returns

fn main() with {alloc, panic} {
    // Temporaries are freed at the end of their statement.
    print(*Box::new(1));                // expect: 1
    Box::new(2);
    print(*two(3).b);                   // expect: 4
    print(first(two(5)));               // expect: 5

    // Boxes in boxes in enums.
    let nested = Option::Some(Box::new(Two { a: Box::new(6), b: Box::new(7) }));
    match nested {
        Option::Some(t) => print(*t.b), // expect: 7
        Option::None => {}
    }

    // Moving one field out, then giving it a new value.
    let mut t = two(8);
    let a = t.a;
    t.a = Box::new(10);
    print(*a + *t.a);                   // expect: 18

    // Matching on a temporary moves out only what's bound.
    match two(11) {
        Two { a, .. } => print(*a),     // expect: 11
    }

    // A new box each time round a loop, and an old one freed by assigning.
    let mut keep = Box::new(0);
    let mut i = 0;
    while i < 5 {
        let fresh = Box::new(i);
        keep = fresh;
        i += 1;
    }
    print(*keep);                       // expect: 4

    // A reference to a box, then the box moved after the reference is done.
    let b = Box::new(12);
    let r = &b;
    print(**r);                         // expect: 12
    let moved = b;
    print(*moved);                      // expect: 12

    // Kept alive by `let`: a reference to a temporary.
    let kept = &two(13);
    print(*kept.a + *kept.b);           // expect: 27
}
