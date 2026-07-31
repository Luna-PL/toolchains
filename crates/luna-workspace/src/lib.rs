//! Luna package/workspace discovery boundary.

use std::path::{Path, PathBuf};

pub const PACKAGE_MANIFEST: &str = "luna.package";
pub const WORKSPACE_MANIFEST: &str = "luna.workspace";
pub const LOCKFILE: &str = "luna.lock";

/// Select the compiler input that owns a saved source file without attempting
/// to reproduce package semantics. The nearest package wins; a workspace is a
/// fallback, and a standalone source remains its own check target.
pub fn compiler_check_target(source: &Path) -> PathBuf {
    let start = if source.is_dir() {
        source
    } else {
        source.parent().unwrap_or_else(|| Path::new("."))
    };
    let mut workspace = None;
    for directory in start.ancestors() {
        if directory.join(PACKAGE_MANIFEST).is_file() {
            return directory.to_path_buf();
        }
        if workspace.is_none() && directory.join(WORKSPACE_MANIFEST).is_file() {
            workspace = Some(directory.to_path_buf());
        }
    }
    workspace.unwrap_or_else(|| source.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    fn temporary_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "luna-workspace-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temporary directory must be creatable");
        path
    }

    #[test]
    fn manifest_names_match_the_compiler_contract() {
        assert_eq!(PACKAGE_MANIFEST, "luna.package");
        assert_eq!(WORKSPACE_MANIFEST, "luna.workspace");
        assert_eq!(LOCKFILE, "luna.lock");
    }

    #[test]
    fn nearest_package_is_the_check_target() {
        let root = temporary_directory();
        fs::write(root.join(WORKSPACE_MANIFEST), b"workspace org.example\n")
            .expect("workspace fixture must be writable");
        let package = root.join("packages/app");
        let source = package.join("src/main.luna");
        fs::create_dir_all(source.parent().expect("source has parent"))
            .expect("package fixture must be creatable");
        fs::write(package.join(PACKAGE_MANIFEST), b"package org.example.app\n")
            .expect("package fixture must be writable");
        fs::write(&source, b"fn main() -> i32 { return 0; }\n")
            .expect("source fixture must be writable");
        assert_eq!(compiler_check_target(&source), package);
        fs::remove_dir_all(root).expect("temporary directory must be removable");
    }

    #[test]
    fn standalone_source_checks_itself() {
        let root = temporary_directory();
        let source = root.join("main.luna");
        fs::write(&source, b"fn main() -> i32 { return 0; }\n")
            .expect("source fixture must be writable");
        assert_eq!(compiler_check_target(&source), source);
        fs::remove_dir_all(root).expect("temporary directory must be removable");
    }
}
