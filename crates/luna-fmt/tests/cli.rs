use luna_compiler::{CandidateSource, check_saved, probe};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn temporary_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "luna-fmt-test-{}-{}",
        std::process::id(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("temporary directory must be creatable");
    path
}

#[test]
fn check_write_and_malformed_boundaries() {
    let root = temporary_directory();
    let source_path = root.join("main.luna");
    let malformed_path = root.join("malformed.luna");
    let source = "fn main( )->i32{ // result\nreturn 0;}\n";
    let malformed = "fn main( { return 0; }\n";
    fs::write(&source_path, source).expect("source fixture must be writable");
    fs::write(&malformed_path, malformed).expect("malformed fixture must be writable");

    let check = Command::new(env!("CARGO_BIN_EXE_luna-fmt"))
        .arg("--check")
        .arg(&source_path)
        .output()
        .expect("formatter check must run");
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&source_path).unwrap(), source);

    let write = Command::new(env!("CARGO_BIN_EXE_luna-fmt"))
        .arg("--write")
        .arg(&source_path)
        .output()
        .expect("formatter write must run");
    assert!(write.status.success());
    let formatted = fs::read_to_string(&source_path).expect("formatted source must be readable");
    assert!(formatted.contains("// result"));

    let recheck = Command::new(env!("CARGO_BIN_EXE_luna-fmt"))
        .arg("--check")
        .arg(&source_path)
        .output()
        .expect("formatter recheck must run");
    assert!(recheck.status.success());

    let conflicting_modes = Command::new(env!("CARGO_BIN_EXE_luna-fmt"))
        .args(["--check", "--write"])
        .arg(&source_path)
        .output()
        .expect("formatter must reject conflicting modes");
    assert_eq!(conflicting_modes.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&source_path).unwrap(), formatted);

    let mut stdin_format = Command::new(env!("CARGO_BIN_EXE_luna-fmt"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("stdin formatter must run");
    stdin_format
        .stdin
        .take()
        .expect("stdin must be piped")
        .write_all(source.as_bytes())
        .expect("source must be writable to stdin");
    let stdin_output = stdin_format
        .wait_with_output()
        .expect("stdin formatter must exit");
    assert!(stdin_output.status.success());
    assert_eq!(String::from_utf8(stdin_output.stdout).unwrap(), formatted);

    let malformed_result = Command::new(env!("CARGO_BIN_EXE_luna-fmt"))
        .arg("--write")
        .arg(&malformed_path)
        .output()
        .expect("malformed check must run");
    assert_eq!(malformed_result.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&malformed_path).unwrap(), malformed);

    if let Some(luna_bin) = std::env::var_os("LUNA_BIN") {
        let compiler = probe(PathBuf::from(luna_bin).as_path(), CandidateSource::Explicit)
            .expect("configured compiler must probe");
        let report = check_saved(&compiler, &source_path)
            .expect("compiler must accept the formatted source");
        assert!(matches!(
            report.records.last(),
            Some(luna_protocol::Record::Summary { success: true, .. })
        ));
    }

    fs::remove_dir_all(root).expect("temporary directory must be removable");
}
