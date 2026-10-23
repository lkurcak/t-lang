fn main() {
    let src = std::path::Path::new("../../src");
    cc::Build::new()
        .include(src)
        .std("c11")
        .warnings(false)
        .file(src.join("parser.c"))
        .compile("tree-sitter-t");
    println!("cargo:rerun-if-changed=../../src/parser.c");
}
