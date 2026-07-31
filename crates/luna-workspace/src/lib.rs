//! Luna package/workspace discovery boundary.

pub const PACKAGE_MANIFEST: &str = "luna.package";
pub const WORKSPACE_MANIFEST: &str = "luna.workspace";
pub const LOCKFILE: &str = "luna.lock";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_names_match_the_compiler_contract() {
        assert_eq!(PACKAGE_MANIFEST, "luna.package");
        assert_eq!(WORKSPACE_MANIFEST, "luna.workspace");
        assert_eq!(LOCKFILE, "luna.lock");
    }
}
