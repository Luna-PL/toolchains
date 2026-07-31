use luna_compiler::{CandidateSource, check_saved, probe};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn temporary_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "luna-fmt-corpus-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("temporary directory must be creatable");
    path
}

#[test]
fn optional_compiler_corpus_is_idempotent() {
    let Some(source_root) = std::env::var_os("LUNA_SOURCE_DIR") else {
        return;
    };
    let source_root = PathBuf::from(source_root);
    let compiler = std::env::var_os("LUNA_BIN").map(|path| {
        probe(PathBuf::from(path).as_path(), CandidateSource::Explicit)
            .expect("configured compiler must probe")
    });
    let formatted_root = compiler.as_ref().map(|_| temporary_directory());
    for relative in [
        "examples/minimal.luna",
        "examples/generic.luna",
        "examples/fragments.luna",
        "examples/operators.luna",
        "examples/ffi.luna",
        "tests/fixtures/result_match.luna",
        "tests/fixtures/iterator_pipeline.luna",
        "tests/fixtures/dynamic_fragments.luna",
        "tests/fixtures/comparison_operators.luna",
        "tests/fixtures/type_relations.luna",
    ] {
        let path = source_root.join(relative);
        let source = fs::read_to_string(&path).expect("compiler fixture must be readable");
        let formatted = luna_fmt::format_source(&source)
            .unwrap_or_else(|error| panic!("formatter rejected {relative}: {error}"));
        assert_eq!(
            luna_fmt::format_source(&formatted)
                .unwrap_or_else(|error| panic!("formatted {relative} is invalid: {error}")),
            formatted,
            "formatter is not idempotent for {relative}"
        );
        if let (Some(compiler), Some(formatted_root)) = (&compiler, &formatted_root) {
            let formatted_path = formatted_root.join(
                path.file_name()
                    .expect("compiler fixture must have a file name"),
            );
            fs::write(&formatted_path, &formatted)
                .expect("formatted compiler fixture must be writable");
            let report = check_saved(compiler, &formatted_path)
                .unwrap_or_else(|error| panic!("compiler rejected {relative}: {error}"));
            assert!(
                matches!(
                    report.records.last(),
                    Some(luna_protocol::Record::Summary { success: true, .. })
                ),
                "compiler reported errors for formatted {relative}"
            );
        }
    }
    if let Some(formatted_root) = formatted_root {
        fs::remove_dir_all(formatted_root).expect("temporary directory must be removable");
    }
}
