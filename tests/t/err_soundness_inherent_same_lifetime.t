// An inherent impl for `P<'a, 'a>`: its method, called on a `P` whose two
// lifetimes differ, makes them the same, so `x` can't outlive `short`.
#[derive(Discard)]
struct P<'a, 'b> { x: &'a int, y: &'b int }

impl<'a> P<'a, 'a> {
    fn flip(&mut self) {
        let t = self.x;
        self.x = self.y;
        self.y = t;
    }
}

fn main() {
    let long = Box::new(1);
    let x;
    {
        let short = Box::new(2);
        let mut p = P { x: &*long, y: &*short };
        p.flip();
        x = p.x;
    }
    print(*x);
}
// error: `short` doesn't live long enough
