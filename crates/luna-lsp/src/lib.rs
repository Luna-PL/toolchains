//! Minimal saved-file Luna language server.

mod syntax;

use luna_compiler::{
    AnalysisOverlay, Compiler, DiscoveryConfig, analyze_overlay, analyze_overlays, analyze_saved,
    check_saved, discover,
};
use luna_protocol::{AnalysisRecord, Record, Span, SymbolKind};
use luna_workspace::compiler_check_target;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

const CHECK_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Clone, Debug)]
struct Document {
    path: PathBuf,
    text: String,
    version: i64,
    dirty: bool,
}

struct AnalysisCache {
    records: Vec<AnalysisRecord>,
    overlay_versions: HashMap<PathBuf, i64>,
}

struct Server {
    initialized: bool,
    shutdown_requested: bool,
    compiler: Option<Compiler>,
    compiler_error: Option<String>,
    documents: HashMap<String, Document>,
    pending_checks: HashMap<String, Instant>,
    published_by_target: HashMap<PathBuf, HashSet<String>>,
    analysis_by_target: HashMap<PathBuf, AnalysisCache>,
}

impl Server {
    fn new() -> Self {
        Self {
            initialized: false,
            shutdown_requested: false,
            compiler: None,
            compiler_error: None,
            documents: HashMap::new(),
            pending_checks: HashMap::new(),
            published_by_target: HashMap::new(),
            analysis_by_target: HashMap::new(),
        }
    }

    fn handle(&mut self, message: Value, output: &mut impl Write) -> io::Result<Option<i32>> {
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            return Ok(None);
        };
        let id = message.get("id").cloned();
        let parameters = message.get("params").cloned().unwrap_or(Value::Null);

        if method == "initialize" {
            let Some(id) = id else {
                return Ok(None);
            };
            if self.initialized {
                return respond_error(output, id, -32600, "server is already initialized");
            }
            self.configure_compiler(&parameters);
            self.initialized = true;
            let definition_provider = self.compiler.as_ref().is_some_and(|compiler| {
                compiler
                    .identity
                    .analysis_capabilities
                    .iter()
                    .any(|capability| {
                        matches!(
                            capability.as_str(),
                            "call-references"
                                | "method-references"
                                | "type-references"
                                | "trait-references"
                        )
                    })
            });
            let references_provider = self.compiler.as_ref().is_some_and(|compiler| {
                compiler
                    .identity
                    .analysis_capabilities
                    .iter()
                    .any(|capability| capability == "package-references")
            });
            return respond(
                output,
                id,
                json!({
                    "capabilities": {
                        "positionEncoding": "utf-16",
                        "textDocumentSync": {
                            "openClose": true,
                            "change": 2,
                            "save": {"includeText": false}
                        },
                        "documentSymbolProvider": true,
                        "foldingRangeProvider": true,
                        "definitionProvider": definition_provider,
                        "referencesProvider": references_provider
                    },
                    "serverInfo": {"name": "luna-lsp", "version": "0.1.0"}
                }),
            );
        }

        if method == "exit" {
            return Ok(Some(if self.shutdown_requested { 0 } else { 1 }));
        }
        if !self.initialized {
            if let Some(id) = id {
                return respond_error(output, id, -32002, "server is not initialized");
            }
            return Ok(None);
        }
        if method == "shutdown" {
            let Some(id) = id else {
                return Ok(None);
            };
            self.shutdown_requested = true;
            return respond(output, id, Value::Null);
        }
        if self.shutdown_requested {
            if let Some(id) = id {
                return respond_error(output, id, -32600, "server is shutting down");
            }
            return Ok(None);
        }

        match method {
            "initialized" => self.report_compiler(output)?,
            "textDocument/didOpen" => self.did_open(&parameters),
            "textDocument/didChange" => {
                if let Err(error) = self.did_change(&parameters) {
                    notify_log(output, 1, &error)?;
                }
            }
            "textDocument/didSave" => self.did_save(&parameters),
            "textDocument/didClose" => self.did_close(&parameters, output)?,
            "textDocument/documentSymbol" => {
                if let Some(id) = id {
                    return respond(output, id, self.symbols(&parameters));
                }
            }
            "textDocument/foldingRange" => {
                if let Some(id) = id {
                    return respond(output, id, self.folding(&parameters));
                }
            }
            "textDocument/definition" => {
                if let Some(id) = id {
                    return respond(output, id, self.definition(&parameters));
                }
            }
            "textDocument/references" => {
                if let Some(id) = id {
                    return respond(output, id, self.references(&parameters));
                }
            }
            "$/cancelRequest" | "workspace/didChangeConfiguration" => {}
            _ => {
                if let Some(id) = id {
                    return respond_error(output, id, -32601, "method not found");
                }
            }
        }
        Ok(None)
    }

    fn configure_compiler(&mut self, parameters: &Value) {
        let mut config = DiscoveryConfig::from_environment();
        if let Some(path) = parameters
            .pointer("/initializationOptions/lunaPath")
            .and_then(Value::as_str)
        {
            config.explicit = Some(PathBuf::from(path));
        }
        match discover(&config) {
            Ok(compiler) => {
                self.compiler = Some(compiler);
                self.compiler_error = None;
            }
            Err(error) => {
                self.compiler = None;
                self.compiler_error = Some(error.to_string());
            }
        }
    }

    fn report_compiler(&self, output: &mut impl Write) -> io::Result<()> {
        if let Some(compiler) = &self.compiler {
            notify_log(
                output,
                3,
                &format!(
                    "using Luna {} ({}, protocol v{}) at {}",
                    compiler.identity.language_version,
                    compiler.identity.compiler_commit,
                    compiler.identity.diagnostic_protocol_version,
                    compiler.executable.display()
                ),
            )
        } else {
            notify_log(
                output,
                1,
                self.compiler_error
                    .as_deref()
                    .unwrap_or("no compatible Luna compiler is configured"),
            )
        }
    }

    fn did_open(&mut self, parameters: &Value) {
        let Some(document) = parameters.get("textDocument") else {
            return;
        };
        let Some(uri) = document.get("uri").and_then(Value::as_str) else {
            return;
        };
        let Some(path) = file_uri_to_path(uri) else {
            return;
        };
        let text = document
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let version = document.get("version").and_then(Value::as_i64).unwrap_or(0);
        let matches_disk = std::fs::read_to_string(&path).is_ok_and(|saved| saved == text);
        let target = compiler_check_target(&path);
        self.documents.insert(
            uri.to_owned(),
            Document {
                path,
                text,
                version,
                dirty: !matches_disk,
            },
        );
        if !matches_disk {
            self.analysis_by_target.remove(&target);
        }
        self.schedule(uri);
    }

    fn did_change(&mut self, parameters: &Value) -> Result<(), String> {
        let document = parameters
            .get("textDocument")
            .ok_or_else(|| "didChange is missing textDocument".to_owned())?;
        let uri = document
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| "didChange is missing URI".to_owned())?;
        let open = self
            .documents
            .get_mut(uri)
            .ok_or_else(|| format!("didChange received for unopened document {uri}"))?;
        let changes = parameters
            .get("contentChanges")
            .and_then(Value::as_array)
            .ok_or_else(|| "didChange is missing contentChanges".to_owned())?;
        syntax::apply_content_changes(&mut open.text, changes)?;
        open.version = document
            .get("version")
            .and_then(Value::as_i64)
            .unwrap_or(open.version);
        open.dirty = true;
        let target = compiler_check_target(&open.path);
        self.analysis_by_target.remove(&target);
        self.schedule(uri);
        Ok(())
    }

    fn did_save(&mut self, parameters: &Value) {
        let Some(uri) = parameters
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return;
        };
        if let Some(document) = self.documents.get_mut(uri) {
            if let Some(text) = parameters.get("text").and_then(Value::as_str) {
                document.text = text.to_owned();
            }
            document.dirty = false;
            let target = compiler_check_target(&document.path);
            self.analysis_by_target.remove(&target);
            self.schedule(uri);
        }
    }

    fn did_close(&mut self, parameters: &Value, output: &mut impl Write) -> io::Result<()> {
        let Some(uri) = parameters
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Ok(());
        };
        if let Some(document) = self.documents.remove(uri) {
            let target = compiler_check_target(&document.path);
            let document_path = canonical_document_path(&document.path);
            if self
                .analysis_by_target
                .get(&target)
                .is_some_and(|cache| cache.overlay_versions.contains_key(&document_path))
            {
                self.analysis_by_target.remove(&target);
            }
        }
        self.pending_checks.remove(uri);
        publish_diagnostics(output, uri, Vec::new(), None)
    }

    fn schedule(&mut self, uri: &str) {
        self.pending_checks
            .insert(uri.to_owned(), Instant::now() + CHECK_DEBOUNCE);
    }

    fn next_timeout(&self) -> Option<Duration> {
        let now = Instant::now();
        self.pending_checks
            .values()
            .min()
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    fn process_due(&mut self, output: &mut impl Write) -> io::Result<()> {
        let now = Instant::now();
        let due: Vec<String> = self
            .pending_checks
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(uri, _)| uri.clone())
            .collect();
        for uri in due {
            self.pending_checks.remove(&uri);
            self.check_document(&uri, output)?;
        }
        Ok(())
    }

    fn check_document(&mut self, uri: &str, output: &mut impl Write) -> io::Result<()> {
        let Some(document) = self.documents.get(uri).cloned() else {
            return Ok(());
        };
        if std::fs::read_to_string(&document.path).is_err() {
            notify_log(output, 1, &format!("cannot read saved Luna source {uri}"))?;
            return Ok(());
        }
        let Some(compiler) = self.compiler.clone() else {
            notify_log(
                output,
                1,
                self.compiler_error
                    .as_deref()
                    .unwrap_or("no compatible Luna compiler is configured"),
            )?;
            return Ok(());
        };
        let target = compiler_check_target(&document.path);
        let mut overlay_documents: Vec<(AnalysisOverlay, i64)> = self
            .documents
            .values()
            .filter(|open| compiler_check_target(&open.path) == target)
            .filter(|open| {
                open.dirty
                    || std::fs::read_to_string(&open.path).is_ok_and(|disk| disk != open.text)
            })
            .map(|open| {
                (
                    AnalysisOverlay {
                        path: open.path.clone(),
                        text: open.text.clone(),
                    },
                    open.version,
                )
            })
            .collect();
        overlay_documents.sort_by(|left, right| left.0.path.cmp(&right.0.path));
        if !overlay_documents.is_empty() {
            let supports_multi = compiler
                .identity
                .analysis_capabilities
                .iter()
                .any(|capability| capability == "multi-document-overlay");
            let supports_single = compiler
                .identity
                .analysis_capabilities
                .iter()
                .any(|capability| capability == "single-document-overlay");
            let overlay_versions = overlay_documents
                .iter()
                .map(|(overlay, version)| (canonical_document_path(&overlay.path), *version))
                .collect();
            let overlays: Vec<AnalysisOverlay> = overlay_documents
                .into_iter()
                .map(|(overlay, _)| overlay)
                .collect();
            let analysis = if supports_multi {
                analyze_overlays(&compiler, &target, &overlays)
            } else if supports_single && overlays.len() == 1 {
                analyze_overlay(&compiler, &target, &overlays[0].path, &overlays[0].text)
            } else {
                notify_log(
                    output,
                    3,
                    &format!(
                        "skipping {} unsaved Luna documents for {uri}",
                        overlays.len()
                    ),
                )?;
                return Ok(());
            };
            match analysis {
                Ok(analysis)
                    if matches!(
                        analysis.records.last(),
                        Some(AnalysisRecord::Summary { complete: true, .. })
                    ) =>
                {
                    self.analysis_by_target.insert(
                        target,
                        AnalysisCache {
                            records: analysis.records,
                            overlay_versions,
                        },
                    );
                }
                Ok(_) => {
                    self.analysis_by_target.remove(&target);
                }
                Err(error) => {
                    self.analysis_by_target.remove(&target);
                    notify_log(output, 2, &format!("Luna overlay analysis failed: {error}"))?;
                }
            }
            return Ok(());
        }
        let report = match check_saved(&compiler, &target) {
            Ok(report) => report,
            Err(error) => {
                notify_log(output, 1, &format!("Luna check failed: {error}"))?;
                return Ok(());
            }
        };

        if compiler.identity.analysis_protocol_version.is_some() {
            match analyze_saved(&compiler, &target) {
                Ok(analysis)
                    if matches!(
                        analysis.records.last(),
                        Some(AnalysisRecord::Summary { complete: true, .. })
                    ) =>
                {
                    self.analysis_by_target.insert(
                        target.clone(),
                        AnalysisCache {
                            records: analysis.records,
                            overlay_versions: HashMap::new(),
                        },
                    );
                }
                Ok(_) => {
                    self.analysis_by_target.remove(&target);
                }
                Err(error) => {
                    self.analysis_by_target.remove(&target);
                    notify_log(output, 2, &format!("Luna analysis failed: {error}"))?;
                }
            }
        }

        let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();
        for record in report.records {
            let Record::Diagnostic {
                severity,
                phase,
                code,
                message,
                primary,
                notes,
                ..
            } = record
            else {
                continue;
            };
            let diagnostic_uri = primary
                .as_ref()
                .map(|span| uri_for_path(&span.path, &self.documents))
                .unwrap_or_else(|| uri.to_owned());
            let range = primary.as_ref().map(lsp_range).unwrap_or_else(zero_range);
            let mut rendered_message = message;
            if !notes.is_empty() {
                rendered_message.push_str("\n\n");
                rendered_message.push_str(&notes.join("\n"));
            }
            grouped.entry(diagnostic_uri).or_default().push(json!({
                "range": range,
                "severity": if severity == "warning" { 2 } else { 1 },
                "code": code,
                "source": "luna",
                "message": rendered_message,
                "data": {"phase": phase}
            }));
        }

        let previous = self.published_by_target.remove(&target).unwrap_or_default();
        let current: HashSet<String> = grouped.keys().cloned().collect();
        for stale in previous.difference(&current) {
            publish_diagnostics(output, stale, Vec::new(), self.document_version(stale))?;
        }
        for (diagnostic_uri, diagnostics) in grouped {
            publish_diagnostics(
                output,
                &diagnostic_uri,
                diagnostics,
                self.document_version(&diagnostic_uri),
            )?;
        }
        if current.is_empty() {
            publish_diagnostics(output, uri, Vec::new(), Some(document.version))?;
        }
        self.published_by_target.insert(target, current);
        Ok(())
    }

    fn document_version(&self, uri: &str) -> Option<i64> {
        self.documents.get(uri).map(|document| document.version)
    }

    fn symbols(&self, parameters: &Value) -> Value {
        let Some(uri) = parameters
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Value::Array(Vec::new());
        };
        let Some(document) = self.documents.get(uri) else {
            return Value::Array(Vec::new());
        };
        if let Some(records) = self.analysis_records_for(document) {
            return Value::Array(analysis_document_symbols(records, document));
        }
        Value::Array(syntax::document_symbols(&document.text))
    }

    fn folding(&self, parameters: &Value) -> Value {
        self.document_text(parameters)
            .map(|source| Value::Array(syntax::folding_ranges(source)))
            .unwrap_or_else(|| Value::Array(Vec::new()))
    }

    fn definition(&self, parameters: &Value) -> Value {
        let Some(uri) = parameters
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Value::Null;
        };
        let Some(document) = self.documents.get(uri) else {
            return Value::Null;
        };
        let Some(position) = parameters.get("position") else {
            return Value::Null;
        };
        let Ok(byte) = syntax::lsp_position_to_byte(&document.text, position) else {
            return Value::Null;
        };
        self.analysis_records_for(document)
            .and_then(|records| {
                definition_from_analysis(records, &document.path, byte, &self.documents)
            })
            .unwrap_or(Value::Null)
    }

    fn references(&self, parameters: &Value) -> Value {
        let Some(uri) = parameters
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)
        else {
            return Value::Array(Vec::new());
        };
        let Some(document) = self.documents.get(uri) else {
            return Value::Array(Vec::new());
        };
        let Some(position) = parameters.get("position") else {
            return Value::Array(Vec::new());
        };
        let Ok(byte) = syntax::lsp_position_to_byte(&document.text, position) else {
            return Value::Array(Vec::new());
        };
        let include_declaration = parameters
            .pointer("/context/includeDeclaration")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.analysis_records_for(document)
            .map(|records| {
                Value::Array(references_from_analysis(
                    records,
                    &document.path,
                    byte,
                    include_declaration,
                    &self.documents,
                ))
            })
            .unwrap_or_else(|| Value::Array(Vec::new()))
    }

    fn analysis_records_for(&self, document: &Document) -> Option<&[AnalysisRecord]> {
        let target = compiler_check_target(&document.path);
        let cache = self.analysis_by_target.get(&target)?;
        let current_overlay_versions: HashMap<PathBuf, i64> = self
            .documents
            .values()
            .filter(|open| compiler_check_target(&open.path) == target)
            .filter(|open| {
                open.dirty
                    || std::fs::read_to_string(&open.path).is_ok_and(|disk| disk != open.text)
            })
            .map(|open| (canonical_document_path(&open.path), open.version))
            .collect();
        if current_overlay_versions != cache.overlay_versions {
            return None;
        }
        Some(&cache.records)
    }

    fn document_text(&self, parameters: &Value) -> Option<&str> {
        let uri = parameters
            .pointer("/textDocument/uri")
            .and_then(Value::as_str)?;
        self.documents
            .get(uri)
            .map(|document| document.text.as_str())
    }
}

fn analysis_document_symbols(records: &[AnalysisRecord], document: &Document) -> Vec<Value> {
    let expected = std::fs::canonicalize(&document.path).unwrap_or_else(|_| document.path.clone());
    records
        .iter()
        .filter_map(|record| {
            let AnalysisRecord::Symbol {
                id,
                name,
                symbol_kind,
                signature,
                selection,
                ..
            } = record
            else {
                return None;
            };
            let actual = std::fs::canonicalize(&selection.path)
                .unwrap_or_else(|_| PathBuf::from(&selection.path));
            if actual != expected {
                return None;
            }
            let range = lsp_range_with_source(selection, &document.text);
            Some(json!({
                "name": name,
                "detail": signature,
                "kind": lsp_symbol_kind(*symbol_kind),
                "range": range,
                "selectionRange": range,
                "data": {"symbolId": id}
            }))
        })
        .collect()
}

fn definition_from_analysis(
    records: &[AnalysisRecord],
    source_path: &Path,
    byte: usize,
    documents: &HashMap<String, Document>,
) -> Option<Value> {
    let expected = std::fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let target_id = records.iter().find_map(|record| {
        let AnalysisRecord::Reference {
            target_id, source, ..
        } = record
        else {
            return None;
        };
        let actual =
            std::fs::canonicalize(&source.path).unwrap_or_else(|_| PathBuf::from(&source.path));
        (actual == expected
            && source.start.byte as usize <= byte
            && byte < source.end.byte as usize)
            .then_some(target_id)
    })?;
    records.iter().find_map(|record| {
        let AnalysisRecord::Symbol { id, selection, .. } = record else {
            return None;
        };
        (id == target_id).then(|| {
            json!({
                "uri": uri_for_path(&selection.path, documents),
                "range": lsp_range_for_documents(selection, documents)
            })
        })
    })
}

fn references_from_analysis(
    records: &[AnalysisRecord],
    source_path: &Path,
    byte: usize,
    include_declaration: bool,
    documents: &HashMap<String, Document>,
) -> Vec<Value> {
    let Some(target_id) = symbol_id_at_analysis_position(records, source_path, byte) else {
        return Vec::new();
    };
    let mut locations = Vec::new();
    if include_declaration
        && let Some(selection) = records.iter().find_map(|record| {
            let AnalysisRecord::Symbol { id, selection, .. } = record else {
                return None;
            };
            (id == target_id).then_some(selection)
        })
    {
        locations.push(analysis_location(selection, documents));
    }
    locations.extend(records.iter().filter_map(|record| {
        let AnalysisRecord::Reference {
            target_id: reference_target,
            source,
            ..
        } = record
        else {
            return None;
        };
        (reference_target == target_id).then(|| analysis_location(source, documents))
    }));
    locations
}

fn symbol_id_at_analysis_position<'a>(
    records: &'a [AnalysisRecord],
    source_path: &Path,
    byte: usize,
) -> Option<&'a str> {
    let expected = canonical_document_path(source_path);
    records
        .iter()
        .find_map(|record| {
            let AnalysisRecord::Reference {
                target_id, source, ..
            } = record
            else {
                return None;
            };
            span_contains_byte(source, &expected, byte).then_some(target_id.as_str())
        })
        .or_else(|| {
            records.iter().find_map(|record| {
                let AnalysisRecord::Symbol { id, selection, .. } = record else {
                    return None;
                };
                span_contains_byte(selection, &expected, byte).then_some(id.as_str())
            })
        })
}

fn span_contains_byte(span: &Span, expected_path: &Path, byte: usize) -> bool {
    canonical_document_path(Path::new(&span.path)) == expected_path
        && span.start.byte as usize <= byte
        && byte < span.end.byte as usize
}

fn analysis_location(span: &Span, documents: &HashMap<String, Document>) -> Value {
    json!({
        "uri": uri_for_path(&span.path, documents),
        "range": lsp_range_for_documents(span, documents)
    })
}

fn lsp_symbol_kind(kind: SymbolKind) -> u32 {
    match kind {
        SymbolKind::Function | SymbolKind::Kernel | SymbolKind::Fragment => 12,
        SymbolKind::Method => 6,
        SymbolKind::Struct => 23,
        SymbolKind::Enum => 10,
        SymbolKind::Trait | SymbolKind::Constraint => 11,
        SymbolKind::Metadata => 19,
    }
}

pub fn run_stdio() -> i32 {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let input = io::stdin();
        let mut reader = BufReader::new(input.lock());
        loop {
            match read_message(&mut reader) {
                Ok(Some(message)) => {
                    if sender.send(Ok(message)).is_err() {
                        return;
                    }
                }
                Ok(None) => {
                    let _ = sender.send(Err(None));
                    return;
                }
                Err(error) => {
                    let _ = sender.send(Err(Some(error)));
                    return;
                }
            }
        }
    });

    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut server = Server::new();
    loop {
        let received = match server.next_timeout() {
            Some(timeout) => receiver.recv_timeout(timeout),
            None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(Ok(message)) => match server.handle(message, &mut output) {
                Ok(Some(status)) => return status,
                Ok(None) => {}
                Err(_) => return 1,
            },
            Ok(Err(Some(_error))) => return 1,
            Ok(Err(None)) | Err(RecvTimeoutError::Disconnected) => {
                return if server.shutdown_requested { 0 } else { 1 };
            }
            Err(RecvTimeoutError::Timeout) => {
                if server.process_due(&mut output).is_err() {
                    return 1;
                }
            }
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut content_length = None;
    let mut saw_header = false;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return if saw_header {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete LSP header",
                ))
            } else {
                Ok(None)
            };
        }
        saw_header = true;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid length"))?,
            );
        }
    }
    let length = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

fn respond(output: &mut impl Write, id: Value, result: Value) -> io::Result<Option<i32>> {
    write_message(
        output,
        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
    )?;
    Ok(None)
}

fn respond_error(
    output: &mut impl Write,
    id: Value,
    code: i32,
    message: &str,
) -> io::Result<Option<i32>> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": code, "message": message}
        }),
    )?;
    Ok(None)
}

fn notify_log(output: &mut impl Write, message_type: u32, message: &str) -> io::Result<()> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "method": "window/logMessage",
            "params": {"type": message_type, "message": message}
        }),
    )
}

fn publish_diagnostics(
    output: &mut impl Write,
    uri: &str,
    diagnostics: Vec<Value>,
    version: Option<i64>,
) -> io::Result<()> {
    let mut parameters = json!({"uri": uri, "diagnostics": diagnostics});
    if let Some(version) = version {
        parameters["version"] = json!(version);
    }
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": parameters
        }),
    )
}

fn lsp_range(span: &Span) -> Value {
    let source = std::fs::read_to_string(&span.path).unwrap_or_default();
    lsp_range_with_source(span, &source)
}

fn lsp_range_for_documents(span: &Span, documents: &HashMap<String, Document>) -> Value {
    let expected = std::fs::canonicalize(&span.path).unwrap_or_else(|_| PathBuf::from(&span.path));
    if let Some(document) = documents.values().find(|document| {
        std::fs::canonicalize(&document.path).unwrap_or_else(|_| document.path.clone()) == expected
    }) {
        return lsp_range_with_source(span, &document.text);
    }
    lsp_range(span)
}

fn lsp_range_with_source(span: &Span, source: &str) -> Value {
    json!({
        "start": byte_to_position(source, span.start.byte as usize, span.start.line, span.start.column),
        "end": byte_to_position(source, span.end.byte as usize, span.end.line, span.end.column)
    })
}

fn byte_to_position(source: &str, byte: usize, fallback_line: u32, fallback_column: u32) -> Value {
    if byte > source.len() {
        return json!({
            "line": fallback_line.saturating_sub(1),
            "character": fallback_column.saturating_sub(1)
        });
    }
    let mut boundary = byte;
    while !source.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    let mut line = 0_u32;
    let mut character = 0_u32;
    for current in source[..boundary].chars() {
        if current == '\n' {
            line += 1;
            character = 0;
        } else {
            character += current.len_utf16() as u32;
        }
    }
    json!({"line": line, "character": character})
}

fn zero_range() -> Value {
    json!({
        "start": {"line": 0, "character": 0},
        "end": {"line": 0, "character": 0}
    })
}

fn uri_for_path(path: &str, documents: &HashMap<String, Document>) -> String {
    let diagnostic_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    documents
        .iter()
        .find(|(_, document)| {
            std::fs::canonicalize(&document.path).unwrap_or_else(|_| document.path.clone())
                == diagnostic_path
        })
        .map(|(uri, _)| uri.clone())
        .unwrap_or_else(|| path_to_file_uri(&diagnostic_path))
}

fn canonical_document_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let encoded = encoded.strip_prefix("localhost").unwrap_or(encoded);
    let decoded = percent_decode(encoded)?;
    #[cfg(windows)]
    let decoded = if decoded.starts_with('/') && decoded.as_bytes().get(2) == Some(&b':') {
        decoded[1..].to_owned()
    } else {
        decoded
    };
    Some(PathBuf::from(decoded))
}

fn path_to_file_uri(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let prefix = if path.starts_with('/') {
        "file://"
    } else {
        "file:///"
    };
    format!("{prefix}{}", percent_encode(&path))
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn percent_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/' | b':') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_and_writes_lsp_frames() {
        let message = json!({"jsonrpc": "2.0", "id": 1, "method": "shutdown"});
        let mut bytes = Vec::new();
        write_message(&mut bytes, &message).expect("frame must serialize");
        let mut reader = BufReader::new(Cursor::new(bytes));
        assert_eq!(
            read_message(&mut reader).expect("frame must parse"),
            Some(message)
        );
    }

    #[test]
    fn file_uri_round_trip_preserves_utf8_paths() {
        let path = Path::new("/tmp/Luna 月/main.luna");
        let uri = path_to_file_uri(path);
        assert_eq!(file_uri_to_path(&uri).as_deref(), Some(path));
    }

    #[test]
    fn byte_offsets_convert_to_utf16() {
        let source = "a🌙b\n";
        assert_eq!(
            byte_to_position(source, 5, 1, 1),
            json!({"line": 0, "character": 3})
        );
    }

    #[test]
    fn compiler_analysis_symbols_preserve_identity_and_signature() {
        let records = luna_protocol::parse_analysis_jsonl(include_str!(
            "../../../tests/protocol/analysis_v1.jsonl"
        ))
        .expect("analysis golden stream must parse");
        let document = Document {
            path: PathBuf::from("/workspace/main.luna"),
            text: "fn add".to_owned(),
            version: 1,
            dirty: false,
        };
        let symbols = analysis_document_symbols(&records, &document);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0]["name"], "add");
        assert_eq!(symbols[0]["kind"], 12);
        assert_eq!(symbols[0]["detail"], "fn add(left: i32, right: i32) -> i32");
        assert_eq!(
            symbols[0].pointer("/data/symbolId"),
            Some(&json!("luna.symbol.v1:4:main:0::8:function:9:main::add"))
        );
    }

    #[test]
    fn compiler_reference_resolves_definition_by_symbol_id() {
        let records = luna_protocol::parse_analysis_jsonl(include_str!(
            "../../../tests/protocol/analysis_v1.jsonl"
        ))
        .expect("analysis golden stream must parse");
        let definition = definition_from_analysis(
            &records,
            Path::new("/workspace/main.luna"),
            59,
            &HashMap::new(),
        )
        .expect("reference must resolve");
        assert_eq!(definition["uri"], "file:///workspace/main.luna");
        assert_eq!(definition.pointer("/range/start/line"), Some(&json!(0)));
        assert_eq!(
            definition.pointer("/range/start/character"),
            Some(&json!(3))
        );
    }

    #[test]
    fn compiler_references_resolve_from_declaration_and_reference() {
        let records = luna_protocol::parse_analysis_jsonl(include_str!(
            "../../../tests/protocol/analysis_v1.jsonl"
        ))
        .expect("analysis golden stream must parse");
        let from_declaration = references_from_analysis(
            &records,
            Path::new("/workspace/main.luna"),
            4,
            false,
            &HashMap::new(),
        );
        assert_eq!(from_declaration.len(), 1);
        assert_eq!(
            from_declaration[0].pointer("/range/start/line"),
            Some(&json!(2))
        );
        let from_reference = references_from_analysis(
            &records,
            Path::new("/workspace/main.luna"),
            59,
            true,
            &HashMap::new(),
        );
        assert_eq!(from_reference.len(), 2);
        assert_eq!(
            from_reference[0].pointer("/range/start/character"),
            Some(&json!(3))
        );
    }
}
