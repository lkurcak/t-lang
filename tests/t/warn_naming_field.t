// Fields are named in snake_case. With a shorthand, `P { xPos }`, the
// fix for the field writes `P { x_pos: xPos }`, and the one for the
// variable `P { x_pos: x_pos }`.
#[derive(Discard)]
struct P { xPos: i64, y: i64 }

fn main() {
    let xPos = 1;
    let p = P { xPos, y: 2 };
    let P { xPos, y } = p;
    print(xPos + y + p.xPos);
}
// warning: field `xPos` should have a snake case name: `x_pos`
// warning: variable `xPos` should have a snake case name: `x_pos`
// warning: variable `xPos` should have a snake case name: `x_pos`
// expect: 4
