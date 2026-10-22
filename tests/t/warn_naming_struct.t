// Structs are named in UpperCamelCase. The fix renames every use too, in
// types, literals and patterns.
#[derive(Discard)]
struct my_point { x: i64, y: i64 }
#[derive(Discard)]
struct HTTPServer { port: i64 }
#[derive(Discard)]
struct Vec2 { x: i64, y: i64 }

fn shift(p: my_point) -> my_point with panic {
    let my_point { x, y } = p;
    my_point { x: x + 1, y }
}

fn main() with panic {
    let p = shift(my_point { x: 1, y: 2 });
    let s = HTTPServer { port: 80 };
    let v = Vec2 { x: 3, y: 4 };
    print(p.x + p.y + s.port + v.x + v.y);
}
// warning: type `my_point` should have an upper camel case name: `MyPoint`
// warning: type `HTTPServer` should have an upper camel case name: `HttpServer`
// expect: 91
