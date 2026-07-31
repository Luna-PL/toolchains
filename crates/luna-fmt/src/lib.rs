//! Comment-preserving full-document formatter for Luna source.

use std::error::Error;
use std::fmt;
use tree_sitter::{Node, Parser};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatError {
    pub message: String,
    pub byte: usize,
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.byte)
    }
}

impl Error for FormatError {}

#[derive(Clone, Debug)]
struct Token<'a> {
    kind: &'a str,
    text: &'a str,
}

pub fn format_source(source: &str) -> Result<String, FormatError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_luna::LANGUAGE.into())
        .map_err(|error| FormatError {
            message: format!("cannot load Luna grammar: {error}"),
            byte: 0,
        })?;
    let tree = parser.parse(source, None).ok_or_else(|| FormatError {
        message: "Tree-sitter did not produce a syntax tree".to_owned(),
        byte: 0,
    })?;
    if let Some(node) = first_error(tree.root_node()) {
        return Err(FormatError {
            message: if node.is_missing() {
                format!("missing {}", node.kind())
            } else {
                "malformed Luna syntax".to_owned()
            },
            byte: node.start_byte(),
        });
    }

    let mut tokens = Vec::new();
    collect_tokens(tree.root_node(), source, &mut tokens);
    Ok(Formatter::new().format(&tokens))
}

fn first_error(node: Node<'_>) -> Option<Node<'_>> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(error) = first_error(child) {
            return Some(error);
        }
    }
    None
}

fn collect_tokens<'a>(node: Node<'_>, source: &'a str, tokens: &mut Vec<Token<'a>>) {
    if node.child_count() == 0 {
        let text = &source[node.byte_range()];
        if !text.trim().is_empty() {
            tokens.push(Token {
                kind: node.kind(),
                text,
            });
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tokens(child, source, tokens);
    }
}

struct Formatter {
    output: String,
    indent: usize,
    line_start: bool,
    parentheses: usize,
    previous: Option<String>,
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            line_start: true,
            parentheses: 0,
            previous: None,
        }
    }

    fn format(mut self, tokens: &[Token<'_>]) -> String {
        for (index, token) in tokens.iter().enumerate() {
            let next = tokens.get(index + 1).map(|token| token.text);
            self.token(token, next);
            self.previous = Some(token.text.to_owned());
        }
        self.trim_space();
        while self.output.ends_with("\n\n") {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }

    fn token(&mut self, token: &Token<'_>, next: Option<&str>) {
        let text = token.text;
        if token.kind == "line_comment" {
            if self.line_start {
                self.write_indent();
            } else {
                self.space();
            }
            self.output.push_str(text.trim_end());
            self.newline();
            return;
        }
        match text {
            "{" => {
                if !self.line_start {
                    self.space();
                }
                self.output.push('{');
                self.newline();
                self.indent += 1;
            }
            "}" => {
                self.indent = self.indent.saturating_sub(1);
                if !self.line_start {
                    self.newline();
                }
                self.write_indent();
                self.output.push('}');
                self.line_start = false;
                if !matches!(next, Some(";" | "," | ")" | "]" | "else")) {
                    self.newline();
                    if self.indent == 0 {
                        self.newline();
                    }
                }
            }
            ";" => {
                self.trim_space();
                self.output.push(';');
                if self.parentheses == 0 {
                    self.newline();
                } else {
                    self.space();
                }
            }
            "," => {
                self.trim_space();
                self.output.push(',');
                self.space();
            }
            "(" => {
                if self.previous.as_deref().is_some_and(is_control_keyword) {
                    self.space();
                }
                self.write_indent();
                self.output.push('(');
                self.line_start = false;
                self.parentheses += 1;
            }
            ")" => {
                self.trim_space();
                self.output.push(')');
                self.line_start = false;
                self.parentheses = self.parentheses.saturating_sub(1);
            }
            "[" => {
                self.write_indent();
                self.output.push('[');
                self.line_start = false;
            }
            "]" => {
                self.trim_space();
                self.output.push(']');
                self.line_start = false;
            }
            "." | "::" | "?" => {
                self.trim_space();
                self.output.push_str(text);
                self.line_start = false;
            }
            ":" => {
                self.trim_space();
                self.output.push(':');
                self.space();
            }
            "@" => {
                self.write_indent();
                self.output.push('@');
                self.line_start = false;
            }
            "<" | ">" | "!" | "~" | "&" => {
                self.trim_space();
                self.output.push_str(text);
                self.line_start = false;
            }
            _ if token.kind == "operator" => {
                self.space();
                self.output.push_str(text);
                self.space();
            }
            _ => {
                self.write_indent();
                if self
                    .previous
                    .as_deref()
                    .is_some_and(|previous| needs_word_space(previous, text))
                {
                    self.space();
                }
                self.output.push_str(text);
                self.line_start = false;
            }
        }
    }

    fn write_indent(&mut self) {
        if self.line_start {
            self.output.push_str(&"    ".repeat(self.indent));
            self.line_start = false;
        }
    }

    fn space(&mut self) {
        if !self.line_start && !self.output.ends_with(' ') && !self.output.ends_with('\n') {
            self.output.push(' ');
        }
    }

    fn trim_space(&mut self) {
        while self.output.ends_with(' ') {
            self.output.pop();
        }
    }

    fn newline(&mut self) {
        self.trim_space();
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.line_start = true;
    }
}

fn is_control_keyword(text: &str) -> bool {
    matches!(text, "if" | "while" | "for" | "match" | "select" | "with")
}

fn needs_word_space(previous: &str, current: &str) -> bool {
    let previous_word = previous
        .chars()
        .last()
        .is_some_and(|character| character.is_alphanumeric() || character == '_');
    let current_word_or_literal = current.chars().next().is_some_and(|character| {
        character.is_alphanumeric() || character == '_' || character == '"'
    });
    let previous_closes_expression = previous.ends_with([')', ']', '}', '"']);
    (current_word_or_literal && (previous_word || previous_closes_expression))
        || (previous == "}" && current == "else")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_is_idempotent_and_preserves_comments() {
        let source = "// heading\n@entry(name) export fn main( )->i32{let x=1+2;// sum\nslot context pipeline(value:i32) default log;return x;}\nextern \"C\" fn foreign(value:i32)->i32;\n";
        let formatted = format_source(source).expect("valid source must format");
        assert!(formatted.contains("// heading"));
        assert!(formatted.contains("// sum"));
        assert!(formatted.contains("@entry(name) export fn"));
        assert!(formatted.contains(") default log;"));
        assert!(formatted.contains("extern \"C\" fn foreign"));
        assert_eq!(
            format_source(&formatted).expect("formatted source must remain valid"),
            formatted
        );
    }

    #[test]
    fn malformed_source_is_not_formatted() {
        let error = format_source("fn main( { return 0; }\n")
            .expect_err("unbalanced source must be rejected");
        assert!(error.message.contains("malformed") || error.message.contains("missing"));
    }
}
