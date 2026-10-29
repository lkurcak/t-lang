// Every module on the way must be exported, not just the innermost.
mod a {
    export mod b {
        #[derive(Discard)]
        export struct P { export x: i64 }
    }
}

fn main() { let _p = a::b::P { x: 1 }; }
// error: module `b` is exported, but other packages can't name it: module `a` isn't exported, and no `export use` names it
