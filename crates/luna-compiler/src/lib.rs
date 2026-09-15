//! Discovery and compatibility probing for the external Luna compiler.

use luna_protocol::{
    ANALYSIS_PROTOCOL_VERSION, AnalysisRecord, DIAGNOSTIC_PROTOCOL_VERSION, Record,
    parse_analysis_jsonl, parse_jsonl, validate_analysis_sequence, validate_sequence,
};
use serde_json::json;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateSource {
    Explicit,
    Path,
    DevelopmentFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilerIdentity {
    pub language_version: String,
    pub compiler_commit: String,
    pub build_target: String,
    pub diagnostic_protocol_version: u32,
    pub capabilities: Vec<String>,
    pub analysis_protocol_version: Option<u32>,
    pub analysis_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Compiler {
    pub executable: PathBuf,
    pub source: CandidateSource,
    pub identity: CompilerIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticReport {
    pub records: Vec<Record>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisReport {
    pub records: Vec<AnalysisRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisOverlay {
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug)]
pub enum AnalysisError {
    Unsupported,
    Spawn(std::io::Error),
    Stdin(std::io::Error),
    Failed {
        status: Option<i32>,
        stdout: String,
        stderr: String,
    },
    NonUtf8,
    InvalidJson(String),
    InvalidSequence(String),
    InvalidOverlay(String),
    CompilerChanged,
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(formatter, "compiler does not support luna.analysis v1"),
            Self::Spawn(source) => write!(formatter, "cannot run compiler analysis: {source}"),
            Self::Stdin(source) => {
                write!(formatter, "cannot send compiler analysis overlay: {source}")
            }
            Self::Failed {
                status,
                stdout,
                stderr,
            } => write!(
                formatter,
                "compiler analysis failed with status {status:?}; stdout={stdout:?}; stderr={stderr:?}"
            ),
            Self::NonUtf8 => write!(formatter, "compiler analysis output is not UTF-8"),
            Self::InvalidJson(error) => write!(formatter, "invalid analysis JSONL: {error}"),
            Self::InvalidSequence(error) => write!(formatter, "invalid analysis sequence: {error}"),
            Self::InvalidOverlay(error) => write!(formatter, "invalid analysis overlay: {error}"),
            Self::CompilerChanged => write!(
                formatter,
                "compiler identity changed after discovery; rediscovery is required"
            ),
        }
    }
}

impl Error for AnalysisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn(source) | Self::Stdin(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum CheckError {
    Spawn(std::io::Error),
    Failed {
        status: Option<i32>,
        stdout: String,
        stderr: String,
    },
    NonUtf8,
    InvalidJson(String),
    InvalidSequence(String),
    CompilerChanged,
}

impl fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(source) => write!(formatter, "cannot run compiler check: {source}"),
            Self::Failed {
                status,
                stdout,
                stderr,
            } => write!(
                formatter,
                "compiler check failed with status {status:?}; stdout={stdout:?}; stderr={stderr:?}"
            ),
            Self::NonUtf8 => write!(formatter, "compiler check output is not UTF-8"),
            Self::InvalidJson(error) => write!(formatter, "invalid diagnostic JSONL: {error}"),
            Self::InvalidSequence(error) => {
                write!(formatter, "invalid diagnostic sequence: {error}")
            }
            Self::CompilerChanged => write!(
                formatter,
                "compiler identity changed after discovery; rediscovery is required"
            ),
        }
    }
}

impl Error for CheckError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiscoveryConfig {
    pub explicit: Option<PathBuf>,
    pub path: Option<OsString>,
    pub development_candidates: Vec<PathBuf>,
}

impl DiscoveryConfig {
    pub fn from_environment() -> Self {
        let executable_name = compiler_executable_name();
        let development_candidates = env::current_dir()
            .ok()
            .and_then(|directory| directory.parent().map(Path::to_path_buf))
            .map(|parent| vec![parent.join("build").join(executable_name)])
            .unwrap_or_default();
        Self {
            explicit: env::var_os("LUNA_BIN").map(PathBuf::from),
            path: env::var_os("PATH"),
            development_candidates,
        }
    }
}

#[derive(Debug)]
pub enum DiscoveryError {
    NotFound,
    Rejected {
        path: PathBuf,
        source: CandidateSource,
        error: ProbeError,
    },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(
                formatter,
                "Luna compiler not found; configure LUNA_BIN, add luna to PATH, or build ../build/luna"
            ),
            Self::Rejected {
                path,
                source,
                error,
            } => write!(
                formatter,
                "rejected {source:?} compiler candidate {}: {error}",
                path.display()
            ),
        }
    }
}

impl Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NotFound => None,
            Self::Rejected { error, .. } => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum ProbeError {
    Spawn {
        operation: &'static str,
        source: std::io::Error,
    },
    Failed {
        operation: &'static str,
        status: Option<i32>,
        stdout: String,
        stderr: String,
    },
    NonUtf8 {
        operation: &'static str,
    },
    InvalidVersion(String),
    InvalidProtocolJson(String),
    InvalidProtocolSequence(String),
    MissingHello,
    VersionMismatch {
        version_output: String,
        hello_version: String,
    },
}

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn { operation, source } => {
                write!(formatter, "cannot run compiler {operation}: {source}")
            }
            Self::Failed {
                operation,
                status,
                stdout,
                stderr,
            } => write!(
                formatter,
                "compiler {operation} failed with status {status:?}; stdout={stdout:?}; stderr={stderr:?}"
            ),
            Self::NonUtf8 { operation } => {
                write!(formatter, "compiler {operation} output is not UTF-8")
            }
            Self::InvalidVersion(output) => {
                write!(formatter, "unexpected compiler version output: {output:?}")
            }
            Self::InvalidProtocolJson(error) => {
                write!(formatter, "invalid compiler diagnostic JSONL: {error}")
            }
            Self::InvalidProtocolSequence(error) => {
                write!(formatter, "invalid compiler diagnostic sequence: {error}")
            }
            Self::MissingHello => write!(formatter, "compiler protocol stream has no hello record"),
            Self::VersionMismatch {
                version_output,
                hello_version,
            } => write!(
                formatter,
                "compiler version {version_output:?} disagrees with protocol hello {hello_version:?}"
            ),
        }
    }
}

impl Error for ProbeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn discover(config: &DiscoveryConfig) -> Result<Compiler, DiscoveryError> {
    discover_with(config, probe)
}

fn discover_with<F>(
    config: &DiscoveryConfig,
    probe_candidate: F,
) -> Result<Compiler, DiscoveryError>
where
    F: Fn(&Path, CandidateSource) -> Result<Compiler, ProbeError>,
{
    if let Some(path) = &config.explicit {
        return selected(path, CandidateSource::Explicit, &probe_candidate);
    }

    if let Some(path) = find_in_path(config.path.as_deref()) {
        return selected(&path, CandidateSource::Path, &probe_candidate);
    }

    for path in &config.development_candidates {
        if path.is_file() {
            return selected(path, CandidateSource::DevelopmentFallback, &probe_candidate);
        }
    }
    Err(DiscoveryError::NotFound)
}

fn selected<F>(
    path: &Path,
    source: CandidateSource,
    probe_candidate: &F,
) -> Result<Compiler, DiscoveryError>
where
    F: Fn(&Path, CandidateSource) -> Result<Compiler, ProbeError>,
{
    probe_candidate(path, source).map_err(|error| DiscoveryError::Rejected {
        path: path.to_path_buf(),
        source,
        error,
    })
}

fn find_in_path(path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let executable_name = compiler_executable_name();
    path.and_then(|value| {
        env::split_paths(value)
            .map(|directory| directory.join(&executable_name))
            .find(|candidate| candidate.is_file())
    })
}

fn compiler_executable_name() -> OsString {
    OsString::from(format!("luna{}", env::consts::EXE_SUFFIX))
}

pub fn probe(path: &Path, source: CandidateSource) -> Result<Compiler, ProbeError> {
    let version = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|source| ProbeError::Spawn {
            operation: "version probe",
            source,
        })?;
    if !version.status.success() {
        return Err(failed("version probe", version));
    }
    let version_stdout = String::from_utf8(version.stdout).map_err(|_| ProbeError::NonUtf8 {
        operation: "version probe",
    })?;
    let version_stderr = String::from_utf8(version.stderr).map_err(|_| ProbeError::NonUtf8 {
        operation: "version probe",
    })?;
    if !version_stderr.is_empty() {
        return Err(ProbeError::Failed {
            operation: "version probe",
            status: version.status.code(),
            stdout: version_stdout,
            stderr: version_stderr,
        });
    }
    let language_version = version_stdout
        .trim()
        .strip_prefix("Luna ")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ProbeError::InvalidVersion(version_stdout.clone()))?
        .to_owned();

    let missing_source = env::temp_dir().join(format!(
        "luna-toolchain-protocol-probe-{}-missing.luna",
        std::process::id()
    ));
    let protocol = Command::new(path)
        .arg("check")
        .arg(&missing_source)
        .arg("--message-format=json")
        .output()
        .map_err(|source| ProbeError::Spawn {
            operation: "protocol probe",
            source,
        })?;
    if !matches!(protocol.status.code(), Some(0 | 1)) {
        return Err(failed("protocol probe", protocol));
    }
    let protocol_stdout = String::from_utf8(protocol.stdout).map_err(|_| ProbeError::NonUtf8 {
        operation: "protocol probe",
    })?;
    let protocol_stderr = String::from_utf8(protocol.stderr).map_err(|_| ProbeError::NonUtf8 {
        operation: "protocol probe",
    })?;
    if !protocol_stderr.is_empty() {
        return Err(ProbeError::Failed {
            operation: "protocol probe",
            status: protocol.status.code(),
            stdout: protocol_stdout,
            stderr: protocol_stderr,
        });
    }
    let mut compiler = identity_from_outputs(path, source, &language_version, &protocol_stdout)?;
    if let Some(capabilities) = probe_analysis(path, &missing_source, &compiler.identity) {
        compiler.identity.analysis_protocol_version = Some(ANALYSIS_PROTOCOL_VERSION);
        compiler.identity.analysis_capabilities = capabilities;
    }
    Ok(compiler)
}

fn probe_analysis(
    path: &Path,
    missing_source: &Path,
    expected: &CompilerIdentity,
) -> Option<Vec<String>> {
    let output = Command::new(path)
        .arg("analyze")
        .arg(missing_source)
        .arg("--message-format=json")
        .output()
        .ok()?;
    if !matches!(output.status.code(), Some(0 | 1)) || !output.stderr.is_empty() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let records = parse_analysis_jsonl(&stdout).ok()?;
    validate_analysis_sequence(&records).ok()?;
    let AnalysisRecord::Hello {
        language_version,
        compiler_commit,
        build_target,
        capabilities,
        ..
    } = records.first()?
    else {
        return None;
    };
    if language_version != &expected.language_version
        || compiler_commit != &expected.compiler_commit
        || build_target != &expected.build_target
        || !capabilities
            .iter()
            .any(|capability| capability == "declarations")
    {
        return None;
    }
    Some(capabilities.clone())
}

pub fn check_saved(compiler: &Compiler, input: &Path) -> Result<DiagnosticReport, CheckError> {
    let output = Command::new(&compiler.executable)
        .arg("check")
        .arg(input)
        .arg("--message-format=json")
        .output()
        .map_err(CheckError::Spawn)?;
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(CheckError::Failed {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let stdout = String::from_utf8(output.stdout).map_err(|_| CheckError::NonUtf8)?;
    let stderr = String::from_utf8(output.stderr).map_err(|_| CheckError::NonUtf8)?;
    if !stderr.is_empty() {
        return Err(CheckError::Failed {
            status: output.status.code(),
            stdout,
            stderr,
        });
    }
    let records =
        parse_jsonl(&stdout).map_err(|error| CheckError::InvalidJson(error.to_string()))?;
    validate_sequence(&records).map_err(|error| CheckError::InvalidSequence(error.to_string()))?;
    let Some(Record::Hello {
        language_version,
        compiler_commit,
        build_target,
        ..
    }) = records.first()
    else {
        return Err(CheckError::InvalidSequence("missing hello".to_owned()));
    };
    if language_version != &compiler.identity.language_version
        || compiler_commit != &compiler.identity.compiler_commit
        || build_target != &compiler.identity.build_target
    {
        return Err(CheckError::CompilerChanged);
    }
    Ok(DiagnosticReport { records })
}

pub fn analyze_saved(compiler: &Compiler, input: &Path) -> Result<AnalysisReport, AnalysisError> {
    if !supports_analysis(compiler, "declarations") {
        return Err(AnalysisError::Unsupported);
    }
    let output = Command::new(&compiler.executable)
        .arg("analyze")
        .arg(input)
        .arg("--message-format=json")
        .output()
        .map_err(AnalysisError::Spawn)?;
    parse_analysis_output(compiler, output)
}

pub fn analyze_overlay(
    compiler: &Compiler,
    input: &Path,
    document: &Path,
    source: &str,
) -> Result<AnalysisReport, AnalysisError> {
    if !supports_analysis(compiler, "declarations")
        || !supports_analysis(compiler, "single-document-overlay")
    {
        return Err(AnalysisError::Unsupported);
    }
    let mut child = Command::new(&compiler.executable)
        .arg("analyze")
        .arg(input)
        .arg("--message-format=json")
        .arg("--overlay")
        .arg(document)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(AnalysisError::Spawn)?;
    child
        .stdin
        .take()
        .expect("piped analysis stdin must be available")
        .write_all(source.as_bytes())
        .map_err(AnalysisError::Stdin)?;
    let output = child.wait_with_output().map_err(AnalysisError::Spawn)?;
    parse_analysis_output(compiler, output)
}

pub fn analyze_overlays(
    compiler: &Compiler,
    input: &Path,
    overlays: &[AnalysisOverlay],
) -> Result<AnalysisReport, AnalysisError> {
    if !supports_analysis(compiler, "declarations")
        || !supports_analysis(compiler, "multi-document-overlay")
    {
        return Err(AnalysisError::Unsupported);
    }
    if overlays.is_empty() {
        return Err(AnalysisError::InvalidOverlay(
            "at least one document is required".to_owned(),
        ));
    }
    let envelope = json!({
        "protocol": "luna.overlay",
        "version": 1,
        "overlays": overlays.iter().map(|overlay| json!({
            "path": overlay.path.to_string_lossy(),
            "text": overlay.text,
        })).collect::<Vec<_>>(),
    })
    .to_string();
    let mut child = Command::new(&compiler.executable)
        .arg("analyze")
        .arg(input)
        .arg("--message-format=json")
        .arg("--overlays-from-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(AnalysisError::Spawn)?;
    child
        .stdin
        .take()
        .expect("piped analysis stdin must be available")
        .write_all(envelope.as_bytes())
        .map_err(AnalysisError::Stdin)?;
    let output = child.wait_with_output().map_err(AnalysisError::Spawn)?;
    parse_analysis_output(compiler, output)
}

fn supports_analysis(compiler: &Compiler, capability: &str) -> bool {
    compiler.identity.analysis_protocol_version == Some(ANALYSIS_PROTOCOL_VERSION)
        && compiler
            .identity
            .analysis_capabilities
            .iter()
            .any(|available| available == capability)
}

fn parse_analysis_output(
    compiler: &Compiler,
    output: Output,
) -> Result<AnalysisReport, AnalysisError> {
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(AnalysisError::Failed {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let stdout = String::from_utf8(output.stdout).map_err(|_| AnalysisError::NonUtf8)?;
    let stderr = String::from_utf8(output.stderr).map_err(|_| AnalysisError::NonUtf8)?;
    if !stderr.is_empty() {
        return Err(AnalysisError::Failed {
            status: output.status.code(),
            stdout,
            stderr,
        });
    }
    let records = parse_analysis_jsonl(&stdout)
        .map_err(|error| AnalysisError::InvalidJson(error.to_string()))?;
    validate_analysis_sequence(&records)
        .map_err(|error| AnalysisError::InvalidSequence(error.to_string()))?;
    let Some(AnalysisRecord::Hello {
        language_version,
        compiler_commit,
        build_target,
        ..
    }) = records.first()
    else {
        return Err(AnalysisError::InvalidSequence("missing hello".to_owned()));
    };
    if language_version != &compiler.identity.language_version
        || compiler_commit != &compiler.identity.compiler_commit
        || build_target != &compiler.identity.build_target
    {
        return Err(AnalysisError::CompilerChanged);
    }
    Ok(AnalysisReport { records })
}

fn failed(operation: &'static str, output: std::process::Output) -> ProbeError {
    ProbeError::Failed {
        operation,
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn identity_from_outputs(
    path: &Path,
    source: CandidateSource,
    version_output: &str,
    jsonl: &str,
) -> Result<Compiler, ProbeError> {
    let records =
        parse_jsonl(jsonl).map_err(|error| ProbeError::InvalidProtocolJson(error.to_string()))?;
    validate_sequence(&records)
        .map_err(|error| ProbeError::InvalidProtocolSequence(error.to_string()))?;
    let Some(Record::Hello {
        language_version,
        compiler_commit,
        build_target,
        capabilities,
        ..
    }) = records.first()
    else {
        return Err(ProbeError::MissingHello);
    };
    if language_version != version_output {
        return Err(ProbeError::VersionMismatch {
            version_output: version_output.to_owned(),
            hello_version: language_version.clone(),
        });
    }
    Ok(Compiler {
        executable: std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        source,
        identity: CompilerIdentity {
            language_version: language_version.clone(),
            compiler_commit: compiler_commit.clone(),
            build_target: build_target.clone(),
            diagnostic_protocol_version: DIAGNOSTIC_PROTOCOL_VERSION,
            capabilities: capabilities.clone(),
            analysis_protocol_version: None,
            analysis_capabilities: Vec::new(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const GOLDEN: &str = include_str!("../../../tests/protocol/diagnostics_v1.jsonl");
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    fn fake_compiler(path: &Path, source: CandidateSource) -> Compiler {
        Compiler {
            executable: path.to_path_buf(),
            source,
            identity: CompilerIdentity {
                language_version: "0.3.0".to_owned(),
                compiler_commit: "test".to_owned(),
                build_target: "test-target".to_owned(),
                diagnostic_protocol_version: 1,
                capabilities: vec!["byte-spans".to_owned()],
                analysis_protocol_version: None,
                analysis_capabilities: Vec::new(),
            },
        }
    }

    fn temporary_directory() -> PathBuf {
        let path = env::temp_dir().join(format!(
            "luna-compiler-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temporary directory must be creatable");
        path
    }

    fn luna_source_root() -> PathBuf {
        env::var_os("LUNA_SOURCE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
    }

    fn has_analysis_capability(compiler: &Compiler, expected: &str) -> bool {
        compiler
            .identity
            .analysis_capabilities
            .iter()
            .any(|capability| capability == expected)
    }

    #[test]
    fn explicit_candidate_has_priority() {
        let selected_sources = RefCell::new(Vec::new());
        let config = DiscoveryConfig {
            explicit: Some(PathBuf::from("configured-luna")),
            path: None,
            development_candidates: vec![PathBuf::from("development-luna")],
        };
        let compiler = discover_with(&config, |path, source| {
            selected_sources.borrow_mut().push(source);
            Ok(fake_compiler(path, source))
        })
        .expect("explicit candidate must be selected");
        assert_eq!(compiler.source, CandidateSource::Explicit);
        assert_eq!(*selected_sources.borrow(), vec![CandidateSource::Explicit]);
    }

    #[test]
    fn rejected_explicit_candidate_does_not_fall_back() {
        let config = DiscoveryConfig {
            explicit: Some(PathBuf::from("configured-luna")),
            path: None,
            development_candidates: vec![PathBuf::from("development-luna")],
        };
        let result = discover_with(&config, |_path, _source| {
            Err(ProbeError::InvalidVersion("not Luna".to_owned()))
        });
        assert!(matches!(
            result,
            Err(DiscoveryError::Rejected {
                source: CandidateSource::Explicit,
                ..
            })
        ));
    }

    #[test]
    fn path_precedes_development_fallback() {
        let directory = temporary_directory();
        let path_compiler = directory.join(compiler_executable_name());
        fs::write(&path_compiler, b"fixture").expect("candidate must be writable");
        let development = directory.join("development-luna");
        fs::write(&development, b"fixture").expect("candidate must be writable");
        let config = DiscoveryConfig {
            explicit: None,
            path: Some(env::join_paths([&directory]).expect("test PATH must be valid")),
            development_candidates: vec![development],
        };
        let compiler = discover_with(&config, |path, source| Ok(fake_compiler(path, source)))
            .expect("PATH candidate must be selected");
        assert_eq!(compiler.source, CandidateSource::Path);
        assert_eq!(compiler.executable, path_compiler);
        fs::remove_dir_all(directory).expect("temporary directory must be removable");
    }

    #[test]
    fn development_candidate_is_the_final_fallback() {
        let directory = temporary_directory();
        let development = directory.join("development-luna");
        fs::write(&development, b"fixture").expect("candidate must be writable");
        let config = DiscoveryConfig {
            explicit: None,
            path: None,
            development_candidates: vec![development.clone()],
        };
        let compiler = discover_with(&config, |path, source| Ok(fake_compiler(path, source)))
            .expect("development candidate must be selected");
        assert_eq!(compiler.source, CandidateSource::DevelopmentFallback);
        assert_eq!(compiler.executable, development);
        fs::remove_dir_all(directory).expect("temporary directory must be removable");
    }

    #[test]
    fn parses_compiler_identity_from_protocol_hello() {
        let compiler = identity_from_outputs(
            Path::new("luna"),
            CandidateSource::Explicit,
            "0.3.0",
            GOLDEN,
        )
        .expect("golden identity must be accepted");
        assert_eq!(compiler.identity.compiler_commit, "0123456789abcdef");
        assert_eq!(compiler.identity.diagnostic_protocol_version, 1);
    }

    #[test]
    fn optional_real_compiler_probe() {
        let Some(path) = env::var_os("LUNA_BIN") else {
            return;
        };
        let compiler = probe(Path::new(&path), CandidateSource::Explicit)
            .expect("LUNA_BIN must implement the diagnostic protocol");
        let compatibility: serde_json::Value =
            serde_json::from_str(include_str!("../../../compatibility/luna.json"))
                .expect("compatibility manifest must be valid JSON");
        let luna = &compatibility["luna"];
        assert_eq!(
            compiler.identity.language_version,
            luna["language_version"]
                .as_str()
                .expect("language version must be a string")
        );
        assert_eq!(
            u64::from(compiler.identity.diagnostic_protocol_version),
            luna["diagnostic_protocol"]
                .as_u64()
                .expect("diagnostic protocol must be an integer")
        );
        assert_eq!(
            compiler.identity.analysis_protocol_version.map(u64::from),
            Some(
                luna["analysis_protocol"]
                    .as_u64()
                    .expect("analysis protocol must be an integer")
            )
        );
        for capability in luna["required_diagnostic_capabilities"]
            .as_array()
            .expect("required diagnostic capabilities must be an array")
        {
            let capability = capability
                .as_str()
                .expect("diagnostic capability must be a string");
            assert!(
                compiler
                    .identity
                    .capabilities
                    .iter()
                    .any(|available| available == capability),
                "compiler is missing required diagnostic capability {capability}"
            );
        }
        for capability in luna["required_analysis_capabilities"]
            .as_array()
            .expect("required analysis capabilities must be an array")
        {
            let capability = capability
                .as_str()
                .expect("analysis capability must be a string");
            assert!(
                has_analysis_capability(&compiler, capability),
                "compiler is missing required analysis capability {capability}"
            );
        }

        let source = luna_source_root().join("examples/minimal.luna");
        let report =
            check_saved(&compiler, &source).expect("LUNA_BIN must check a saved standalone source");
        assert!(matches!(
            report.records.last(),
            Some(Record::Summary { success: true, .. })
        ));

        let analysis = analyze_saved(&compiler, &source)
            .expect("LUNA_BIN must analyze a saved standalone source");
        assert!(matches!(
            analysis.records.last(),
            Some(AnalysisRecord::Summary {
                symbols: 1,
                complete: true,
                ..
            })
        ));

        let control_source = luna_source_root().join("examples/fragments.luna");
        let control_analysis = analyze_saved(&compiler, &control_source)
            .expect("LUNA_BIN analysis must accept the open Slot/Fragment symbol vocabulary");
        assert!(control_analysis.records.iter().any(|record| matches!(
            record,
            AnalysisRecord::Symbol {
                symbol_kind: luna_protocol::SymbolKind::Slot,
                ..
            }
        )));
        assert!(control_analysis.records.iter().any(|record| matches!(
            record,
            AnalysisRecord::Symbol {
                symbol_kind: luna_protocol::SymbolKind::Fragment,
                ..
            }
        )));

        if has_analysis_capability(&compiler, "single-document-overlay") {
            let overlaid = analyze_overlay(
                &compiler,
                &source,
                &source,
                "// 月\nfn unsaved() -> i32 { return 5; }\nfn main() -> i32 { return unsaved(); }\n",
            )
            .expect("advertised single-document overlay capability must work");
            assert!(overlaid.records.iter().any(|record| matches!(
                record,
                AnalysisRecord::Symbol { name, .. } if name == "unsaved"
            )));
            if has_analysis_capability(&compiler, "call-references") {
                assert!(
                    overlaid
                        .records
                        .iter()
                        .any(|record| matches!(record, AnalysisRecord::Reference { .. }))
                );
            }
        }

        if has_analysis_capability(&compiler, "multi-document-overlay") {
            let package = luna_source_root().join("tests/fixtures/packages/module_headers");
            let multi_overlaid = analyze_overlays(
                &compiler,
                &package,
                &[
                    AnalysisOverlay {
                        path: package.join("01_math.luna"),
                        text: "package org.luna.module_headers;\nmodule math::integer;\nusing org.luna.std as std;\n// 月\nexport fn moon_answer() -> i32 { return 42; }\n".to_owned(),
                    },
                    AnalysisOverlay {
                        path: package.join("02_main.luna"),
                        text: "package org.luna.module_headers;\nmodule application;\nusing org.luna.std as std;\nfn main() -> i32 { return math::integer::moon_answer(); }\n".to_owned(),
                    },
                ],
            )
            .expect("advertised multi-document overlay capability must work");
            assert!(multi_overlaid.records.iter().any(|record| matches!(
                record,
                AnalysisRecord::Symbol { name, .. } if name == "moon_answer"
            )));
            if has_analysis_capability(&compiler, "call-references") {
                assert!(
                    multi_overlaid
                        .records
                        .iter()
                        .any(|record| matches!(record, AnalysisRecord::Reference { .. }))
                );
            }
        }
    }
}
