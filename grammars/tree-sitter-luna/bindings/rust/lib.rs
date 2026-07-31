//! Rust binding for the generated Luna Tree-sitter parser.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_luna() -> *const ();
}

pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_luna) };

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_parser_loads() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&LANGUAGE.into())
            .expect("generated Luna language must load");
        let tree = parser
            .parse("fn main() -> i32 { return 0; }", None)
            .expect("source must parse");
        assert!(!tree.root_node().has_error());
    }
}
