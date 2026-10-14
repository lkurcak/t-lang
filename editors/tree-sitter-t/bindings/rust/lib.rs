//! The tree-sitter grammar for T, for syntax highlighting. The compiler
//! doesn't use it: its parser is `src/parser.rs`.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_t() -> *const ();
}

/// The grammar, for `tree_sitter::Parser::set_language`.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_t) };

/// The syntax highlighting query, shared by Helix and Neovim.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");
