// `let` takes any irrefutable pattern: one that matches every value of its type.

struct Point { x: int, y: int }
struct Line { from: Point, to: Point }
enum Wrapper { Only(int) }   // one variant, so matching it can't fail

fn main() {
    let Point { x, y } = Point { x: 1, y: 2 };
    print(x + y);           // expect: 3
    let Line { from: Point { x: x0, .. }, to: Point { y: y1, .. } } =
        Line { from: Point { x: 10, y: 0 }, to: Point { x: 0, y: 20 } };
    print(x0 + y1);         // expect: 30
    let Wrapper::Only(n) = Wrapper::Only(5);
    print(n);               // expect: 5
    let _ = 99;
    let () = ();
}
