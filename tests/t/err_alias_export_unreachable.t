// What's exported must be reachable from the package's root.
mod m {
    export type Num = i64;
}

fn main() {}
// error: `Num` is exported, but other packages can't name it: module `m` isn't exported, and no `export use` names it
