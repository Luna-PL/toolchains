//! Stable wire types for compiler-owned Luna tooling protocols.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

pub const DIAGNOSTIC_PROTOCOL: &str = "luna.diagnostic";
pub const DIAGNOSTIC_PROTOCOL_VERSION: u32 = 1;
pub const ANALYSIS_PROTOCOL: &str = "luna.analysis";
pub const ANALYSIS_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolVersion {
    pub name: &'static str,
    pub version: u32,
}

pub const DIAGNOSTICS_V1: ProtocolVersion = ProtocolVersion {
    name: DIAGNOSTIC_PROTOCOL,
    version: DIAGNOSTIC_PROTOCOL_VERSION,
};

pub const ANALYSIS_V1: ProtocolVersion = ProtocolVersion {
    name: ANALYSIS_PROTOCOL,
    version: ANALYSIS_PROTOCOL_VERSION,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Position {
    pub byte: u64,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Span {
    pub path: String,
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Label {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    Manual,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextEdit {
    pub span: Span,
    pub replacement: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fix {
    pub message: String,
    pub applicability: Applicability,
    pub edits: Vec<TextEdit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Record {
    Hello {
        protocol: String,
        version: u32,
        language_version: String,
        compiler_commit: String,
        build_target: String,
        capabilities: Vec<String>,
    },
    Diagnostic {
        protocol: String,
        version: u32,
        severity: String,
        phase: String,
        code: String,
        message: String,
        primary: Option<Span>,
        labels: Vec<Label>,
        notes: Vec<String>,
        fixes: Vec<Fix>,
    },
    Summary {
        protocol: String,
        version: u32,
        errors: u32,
        warnings: u32,
        success: bool,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SymbolKind {
    Function,
    Kernel,
    Method,
    Fragment,
    Struct,
    Enum,
    Trait,
    Metadata,
    Constraint,
    Field,
    EnumVariant,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AnalysisRecord {
    Hello {
        protocol: String,
        version: u32,
        language_version: String,
        compiler_commit: String,
        build_target: String,
        capabilities: Vec<String>,
    },
    Symbol {
        protocol: String,
        version: u32,
        id: String,
        name: String,
        qualified_name: String,
        package_id: String,
        module_path: String,
        linkage_name: String,
        symbol_kind: SymbolKind,
        signature: String,
        selection: Span,
        exported: bool,
        external: bool,
    },
    Reference {
        protocol: String,
        version: u32,
        target_id: String,
        source: Span,
    },
    Summary {
        protocol: String,
        version: u32,
        symbols: u32,
        references: u32,
        complete: bool,
    },
}

impl AnalysisRecord {
    pub fn protocol(&self) -> &str {
        match self {
            Self::Hello { protocol, .. }
            | Self::Symbol { protocol, .. }
            | Self::Reference { protocol, .. }
            | Self::Summary { protocol, .. } => protocol,
        }
    }

    pub fn version(&self) -> u32 {
        match self {
            Self::Hello { version, .. }
            | Self::Symbol { version, .. }
            | Self::Reference { version, .. }
            | Self::Summary { version, .. } => *version,
        }
    }
}

impl Record {
    pub fn protocol(&self) -> &str {
        match self {
            Self::Hello { protocol, .. }
            | Self::Diagnostic { protocol, .. }
            | Self::Summary { protocol, .. } => protocol,
        }
    }

    pub fn version(&self) -> u32 {
        match self {
            Self::Hello { version, .. }
            | Self::Diagnostic { version, .. }
            | Self::Summary { version, .. } => *version,
        }
    }
}

#[derive(Debug)]
pub struct JsonLineError {
    pub line: usize,
    pub source: serde_json::Error,
}

impl fmt::Display for JsonLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid protocol JSON on line {}: {}",
            self.line, self.source
        )
    }
}

impl Error for JsonLineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

pub fn parse_jsonl(input: &str) -> Result<Vec<Record>, JsonLineError> {
    input
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|source| JsonLineError {
                line: index + 1,
                source,
            })
        })
        .collect()
}

pub fn parse_analysis_jsonl(input: &str) -> Result<Vec<AnalysisRecord>, JsonLineError> {
    input
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|source| JsonLineError {
                line: index + 1,
                source,
            })
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SequenceError {
    Empty,
    UnsupportedIdentity {
        index: usize,
        protocol: String,
        version: u32,
    },
    MissingHello,
    UnexpectedRecord {
        index: usize,
    },
    MissingSummary,
    InconsistentSummary,
    DuplicateSymbolId {
        index: usize,
        id: String,
    },
    UnknownReferenceTarget {
        index: usize,
        target_id: String,
    },
}

impl fmt::Display for SequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "diagnostic stream is empty"),
            Self::UnsupportedIdentity {
                index,
                protocol,
                version,
            } => write!(
                formatter,
                "record {} uses unsupported protocol {protocol} version {version}",
                index + 1
            ),
            Self::MissingHello => write!(formatter, "first record is not hello"),
            Self::UnexpectedRecord { index } => {
                write!(formatter, "record {} is out of sequence", index + 1)
            }
            Self::MissingSummary => write!(formatter, "last record is not summary"),
            Self::InconsistentSummary => {
                write!(formatter, "summary counts do not match stream records")
            }
            Self::DuplicateSymbolId { index, id } => {
                write!(formatter, "record {} repeats symbol id {id}", index + 1)
            }
            Self::UnknownReferenceTarget { index, target_id } => write!(
                formatter,
                "record {} references unknown symbol id {target_id}",
                index + 1
            ),
        }
    }
}

impl Error for SequenceError {}

pub fn validate_sequence(records: &[Record]) -> Result<(), SequenceError> {
    if records.is_empty() {
        return Err(SequenceError::Empty);
    }
    for (index, record) in records.iter().enumerate() {
        if record.protocol() != DIAGNOSTIC_PROTOCOL
            || record.version() != DIAGNOSTIC_PROTOCOL_VERSION
        {
            return Err(SequenceError::UnsupportedIdentity {
                index,
                protocol: record.protocol().to_owned(),
                version: record.version(),
            });
        }
    }
    if !matches!(records.first(), Some(Record::Hello { .. })) {
        return Err(SequenceError::MissingHello);
    }
    if !matches!(records.last(), Some(Record::Summary { .. })) {
        return Err(SequenceError::MissingSummary);
    }
    for (index, record) in records[1..records.len() - 1].iter().enumerate() {
        if !matches!(record, Record::Diagnostic { .. }) {
            return Err(SequenceError::UnexpectedRecord { index: index + 1 });
        }
    }

    let errors = records
        .iter()
        .filter(
            |record| matches!(record, Record::Diagnostic { severity, .. } if severity == "error"),
        )
        .count() as u32;
    let warnings = records
        .iter()
        .filter(
            |record| matches!(record, Record::Diagnostic { severity, .. } if severity == "warning"),
        )
        .count() as u32;
    let Record::Summary {
        errors: summary_errors,
        warnings: summary_warnings,
        success,
        ..
    } = records.last().expect("non-empty sequence checked above")
    else {
        unreachable!("summary shape checked above");
    };
    if errors != *summary_errors || warnings != *summary_warnings || *success != (errors == 0) {
        return Err(SequenceError::InconsistentSummary);
    }
    Ok(())
}

pub fn validate_analysis_sequence(records: &[AnalysisRecord]) -> Result<(), SequenceError> {
    use std::collections::HashSet;

    if records.is_empty() {
        return Err(SequenceError::Empty);
    }
    for (index, record) in records.iter().enumerate() {
        if record.protocol() != ANALYSIS_PROTOCOL || record.version() != ANALYSIS_PROTOCOL_VERSION {
            return Err(SequenceError::UnsupportedIdentity {
                index,
                protocol: record.protocol().to_owned(),
                version: record.version(),
            });
        }
    }
    if !matches!(records.first(), Some(AnalysisRecord::Hello { .. })) {
        return Err(SequenceError::MissingHello);
    }
    if !matches!(records.last(), Some(AnalysisRecord::Summary { .. })) {
        return Err(SequenceError::MissingSummary);
    }

    let mut ids = HashSet::new();
    let mut references = Vec::new();
    for (offset, record) in records[1..records.len() - 1].iter().enumerate() {
        let index = offset + 1;
        match record {
            AnalysisRecord::Symbol { id, .. } => {
                if !ids.insert(id) {
                    return Err(SequenceError::DuplicateSymbolId {
                        index,
                        id: id.clone(),
                    });
                }
            }
            AnalysisRecord::Reference { target_id, .. } => {
                references.push((index, target_id));
            }
            _ => return Err(SequenceError::UnexpectedRecord { index }),
        }
    }
    for (index, target_id) in &references {
        if !ids.contains(target_id) {
            return Err(SequenceError::UnknownReferenceTarget {
                index: *index,
                target_id: (*target_id).clone(),
            });
        }
    }

    let AnalysisRecord::Summary {
        symbols,
        references: summary_references,
        ..
    } = records.last().expect("non-empty sequence checked above")
    else {
        unreachable!("summary shape checked above");
    };
    if *symbols != ids.len() as u32 || *summary_references != references.len() as u32 {
        return Err(SequenceError::InconsistentSummary);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &str = include_str!("../../../tests/protocol/diagnostics_v1.jsonl");
    const ANALYSIS_GOLDEN: &str = include_str!("../../../tests/protocol/analysis_v1.jsonl");

    #[test]
    fn diagnostic_protocol_starts_at_v1() {
        assert_eq!(DIAGNOSTICS_V1.name, "luna.diagnostic");
        assert_eq!(DIAGNOSTICS_V1.version, 1);
        assert_eq!(ANALYSIS_V1.name, "luna.analysis");
        assert_eq!(ANALYSIS_V1.version, 1);
    }

    #[test]
    fn parses_and_validates_v1_golden_stream() {
        let records = parse_jsonl(GOLDEN).expect("golden JSONL must deserialize");
        validate_sequence(&records).expect("golden protocol sequence must validate");
        assert_eq!(records.len(), 3);
        assert!(matches!(
            &records[1],
            Record::Diagnostic {
                code,
                primary: Some(Span { start, end, .. }),
                ..
            } if code == "SEM0001" && start.byte == 42 && end.byte == 47
        ));
    }

    #[test]
    fn rejects_records_after_summary() {
        let mut records = parse_jsonl(GOLDEN).expect("golden JSONL must deserialize");
        records.swap(1, 2);
        assert_eq!(
            validate_sequence(&records),
            Err(SequenceError::MissingSummary)
        );
    }

    #[test]
    fn parses_and_validates_analysis_v1_golden_stream() {
        let records = parse_analysis_jsonl(ANALYSIS_GOLDEN).expect("golden JSONL must deserialize");
        validate_analysis_sequence(&records).expect("golden protocol sequence must validate");
        assert_eq!(records.len(), 4);
        assert!(matches!(
            &records[1],
            AnalysisRecord::Symbol {
                id,
                symbol_kind: SymbolKind::Function,
                selection: Span { start, end, .. },
                ..
            } if id.starts_with("luna.symbol.v1") && start.byte == 3 && end.byte == 7
        ));
        assert!(matches!(
            &records[2],
            AnalysisRecord::Reference {
                target_id,
                source: Span { start, end, .. },
                ..
            } if target_id.starts_with("luna.symbol.v1") && start.byte == 58 && end.byte == 61
        ));
    }

    #[test]
    fn parses_member_symbol_kinds() {
        for (wire_name, expected) in [
            ("field", SymbolKind::Field),
            ("enum-variant", SymbolKind::EnumVariant),
        ] {
            let encoded = format!("\"{wire_name}\"");
            assert_eq!(
                serde_json::from_str::<SymbolKind>(&encoded)
                    .expect("member symbol kind must deserialize"),
                expected
            );
        }
    }

    #[test]
    fn rejects_duplicate_analysis_symbol_ids() {
        let mut records =
            parse_analysis_jsonl(ANALYSIS_GOLDEN).expect("golden JSONL must deserialize");
        let duplicate = records[1].clone();
        records.insert(2, duplicate);
        if let Some(AnalysisRecord::Summary { symbols, .. }) = records.last_mut() {
            *symbols = 2;
        }
        assert!(matches!(
            validate_analysis_sequence(&records),
            Err(SequenceError::DuplicateSymbolId { index: 2, .. })
        ));
    }

    #[test]
    fn rejects_reference_to_unknown_symbol_id() {
        let mut records =
            parse_analysis_jsonl(ANALYSIS_GOLDEN).expect("golden JSONL must deserialize");
        if let AnalysisRecord::Reference { target_id, .. } = &mut records[2] {
            *target_id = "luna.symbol.v1:7:missing".to_owned();
        }
        assert!(matches!(
            validate_analysis_sequence(&records),
            Err(SequenceError::UnknownReferenceTarget { index: 2, .. })
        ));
    }
}
