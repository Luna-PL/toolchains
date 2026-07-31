/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const DECLARATION_KEYWORDS = [
  "fn",
  "struct",
  "enum",
  "trait",
  "impl",
  "interceptor",
  "context",
  "meta",
  "constraint",
];

const MODIFIERS = [
  "export",
  "constexpr",
  "extern",
  "kernel",
  "nominal",
  "runtime",
  "dynamic",
];

module.exports = grammar({
  name: "luna",

  extras: ($) => [/\s/, $.line_comment],

  word: ($) => $.identifier,

  rules: {
    source_file: ($) =>
      repeat(
        choice(
          $.package_header,
          $.module_header,
          $.using_header,
          $.declaration,
        ),
      ),

    package_header: ($) => seq("package", $.package_id, ";"),

    module_header: ($) => seq("module", $.module_path, ";"),

    using_header: ($) =>
      seq("using", $.package_id, "as", $.identifier, ";"),

    package_id: ($) => seq($.identifier, repeat(seq(".", $.identifier))),

    module_path: ($) => seq($.identifier, repeat(seq("::", $.identifier))),

    declaration: ($) =>
      seq(
        repeat(choice($.metadata_attachment, ...MODIFIERS)),
        optional(field("abi", $.string)),
        field("kind", choice(...DECLARATION_KEYWORDS)),
        repeat($._fragment),
        choice($.block, ";"),
      ),

    metadata_attachment: ($) => seq("@", $.identifier, $.parenthesized),

    block: ($) => seq("{", repeat(choice($._fragment, $.block)), "}"),

    parenthesized: ($) =>
      seq("(", repeat(choice($._fragment, $.block)), ")"),

    bracketed: ($) => seq("[", repeat(choice($._fragment, $.block)), "]"),

    _fragment: ($) =>
      choice(
        $.parenthesized,
        $.bracketed,
        $.identifier,
        $.number,
        $.string,
        $.operator,
        $.punctuation,
      ),

    identifier: (_) => /[A-Za-z_][A-Za-z0-9_]*/,

    number: (_) => /[0-9]+(?:\.[0-9]+)?/,

    string: (_) => /"(?:\\.|[^"\\])*"/,

    operator: (_) =>
      token(
        choice(
          "<<=",
          ">>=",
          "->",
          "=>",
          "::",
          "..",
          "+=",
          "-=",
          "*=",
          "/=",
          "%=",
          "&=",
          "|=",
          "^=",
          "==",
          "!=",
          "<=",
          ">=",
          "<<",
          ">>",
          "&&",
          "||",
          "+",
          "-",
          "*",
          "/",
          "%",
          "=",
          "<",
          ">",
          "!",
          "~",
          "&",
          "|",
          "^",
          "?",
        ),
      ),

    punctuation: (_) => token(choice(":", ".", ",", ";")),

    line_comment: (_) => token(seq("//", /[^\n]*/)),
  },
});
