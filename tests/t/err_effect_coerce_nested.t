// A function type gains effects only as a whole value, not inside another
// type: there's no subtyping, so `Option<fn()>` isn't an
// `Option<fn() with panic>`.
fn nothing() {}

fn main() {
    let f: Option<fn()> = Option::Some(nothing);
    let g: Option<fn() with panic> = f;
    print_bool(g.is_some());
}
// error: expected `Option<fn() -> () with panic>`, found `Option<fn() -> ()>`
