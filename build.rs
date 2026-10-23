//! Builds the tree-sitter grammar in `editors/tree-sitter-t` into a shared
//! library, for `t tree-sitter` to install where editors load parsers from.
//! It's built here, with the C compiler that links `t`, so a `t` from a
//! release archive can install it on a machine with no C compiler.

use std::path::PathBuf;

fn main() {
    let src = PathBuf::from("editors/tree-sitter-t/src");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed={}", src.join("parser.c").display());

    // `cc` finds the compiler for the target, and on Windows the environment
    // MSVC needs; it only builds static libraries itself, so ask for a shared
    // one ourselves.
    let compiler = cc::Build::new().opt_level(2).warnings(false).get_compiler();
    let mut cmd = compiler.to_command();
    let out;
    if compiler.is_like_msvc() {
        out = out_dir.join("tree-sitter-t.dll");
        let obj = out_dir.join("parser.obj");
        cmd.arg("/LD")
            .arg("/I")
            .arg(&src)
            .arg(src.join("parser.c"))
            .arg(format!("/Fo{}", obj.display()))
            .arg(format!("/Fe{}", out.display()));
    } else {
        out = out_dir.join("tree-sitter-t.so");
        cmd.args(["-shared", "-fPIC", "-std=c11", "-I"])
            .arg(&src)
            .arg(src.join("parser.c"))
            .arg("-o")
            .arg(&out);
    }
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("cannot run the C compiler: {e}"));
    assert!(status.success(), "building the tree-sitter parser failed");
    println!("cargo:rustc-env=T_TREE_SITTER_PARSER={}", out.display());
}
