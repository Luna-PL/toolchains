fn main() {
    let source = std::path::Path::new("src");
    cc::Build::new()
        .include(source)
        .file(source.join("parser.c"))
        .warnings(false)
        .compile("tree-sitter-luna");
    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/tree_sitter/parser.h");
}
