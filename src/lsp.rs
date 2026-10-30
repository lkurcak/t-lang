//! A Language Server Protocol server, so any LSP-capable editor can show T's
//! errors and warnings as you type, and offer their fixes as quick fixes. Run
//! as `t lsp`; it talks JSON-RPC over stdin/stdout.
//!
//! A file is checked as part of its program: the one of the nearest
//! `main.t` above it that has it as a module's file, or else the file
//! alone (see `load.rs`), with what the editor has for open files and
//! what's on disk for the others. It shares semantic analysis between
//! requests until an input changes, then re-checks the whole program and
//! reports every warning and the first semantic error, each in its file.
//! Syntax errors recover at module item boundaries for diagnostics and the
//! outline only; semantic analysis still requires a strict parse. Requests
//! are answered from what the checker found out
//! (`check::Info`): the fixes of the diagnostics in range as code actions,
//! the type and doc comment of what's under the cursor on hover, where
//! that's defined and where its type is, every place it's named (as
//! references and highlights), and inferred types and type arguments as
//! inlay hints. It renames as `rename.rs` does. The outline of a file comes
//! from parsing it alone, retaining complete declarations around syntax errors.
//! It formats a file like `t fmt`, ignoring the editor's formatting
//! options: there's one style. It never runs the program.

use crate::ast::{FnDecl, MethodOf, Program, ROOT, TypeDecl};
use crate::check::{Info, NoteKind};
use crate::error::{Error, Fix, Lint, Phase, Span, Warning};
use crate::fmt::FmtError;
use crate::source::{File as SourceFile, Sources};
use crate::types::{Type, comma_list};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    DidSaveTextDocument, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, DocumentHighlightRequest, DocumentSymbolRequest, Formatting, GotoDefinition,
    GotoTypeDefinition, HoverRequest, InlayHintRequest, PrepareRenameRequest, References, Rename,
    Request as _,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DiagnosticTag, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentHighlight, DocumentHighlightKind,
    DocumentHighlightParams, DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    HoverProviderCapability, InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, Location,
    MarkupContent, MarkupKind, NumberOrString, OneOf, Position, PrepareRenameResponse,
    PublishDiagnosticsParams, Range, ReferenceParams, RenameOptions, RenameParams,
    ServerCapabilities, SymbolKind, TextDocumentPositionParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, TypeDefinitionProviderCapability, Uri, WorkspaceEdit,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

mod library;
use library::LibrarySources;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn run() -> Result<()> {
    let mut docs = Docs::new()?;
    let (conn, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        // FULL: the editor sends the whole file on every change, not diffs.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![
                CodeActionKind::QUICKFIX,
                CodeActionKind::REFACTOR_REWRITE,
            ]),
            ..Default::default()
        })),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        document_symbol_provider: Some(OneOf::Left(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };
    conn.initialize(serde_json::to_value(capabilities)?)?;

    let mut published = HashMap::new();
    // A message the server can't make sense of, such as one with a URI
    // that isn't one, gets an error, if it's a request, and is otherwise
    // skipped: it mustn't stop the server, and with it the editor's help
    // for every other file.
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    break;
                }
                // Poll dependencies too: clients need not watch files on
                // disk for a request to see their current contents.
                refresh_diagnostics(&conn, &docs, &mut published)?;
                let id = req.id.clone();
                let response = respond(req, &docs).unwrap_or_else(|e| {
                    Response::new_err(id, ErrorCode::InvalidParams as i32, e.to_string())
                });
                conn.sender.send(response.into())?;
            }
            Message::Notification(n) => {
                if let Err(e) = notify(&conn, n, &mut docs, &mut published) {
                    eprintln!("t lsp: {e}");
                }
            }
            Message::Response(_) => {}
        }
    }

    // The writer thread exits once every sender is gone, so `conn` must be
    // dropped before joining or this waits forever.
    drop(conn);
    io_threads.join()?;
    Ok(())
}

/// Handles notification `n`: a file opened, changed or closed.
fn notify(
    conn: &Connection,
    n: Notification,
    docs: &mut Docs,
    published: &mut HashMap<PathBuf, Rc<Analysis>>,
) -> Result<()> {
    match n.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let p: DidOpenTextDocumentParams = serde_json::from_value(n.params)?;
            let doc = p.text_document;
            docs.open(&doc.uri, doc.version, doc.text);
            if docs.is_library(&doc.uri) {
                publish(conn, doc.uri, Some(doc.version), Vec::new())?;
            }
            refresh_diagnostics(conn, docs, published)?;
        }
        DidChangeTextDocument::METHOD => {
            let p: DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
            // With FULL sync, the last change holds the whole new text.
            if let Some(change) = p.content_changes.into_iter().last() {
                let doc = p.text_document;
                docs.open(&doc.uri, doc.version, change.text);
                if docs.is_library(&doc.uri) {
                    publish(conn, doc.uri, Some(doc.version), Vec::new())?;
                }
                refresh_diagnostics(conn, docs, published)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let p: DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
            let uri = p.text_document.uri;
            docs.files.remove(&uri_path(&uri));
            publish(conn, uri, None, Vec::new())?;
            refresh_diagnostics(conn, docs, published)?;
        }
        DidChangeWatchedFiles::METHOD | DidSaveTextDocument::METHOD => {
            refresh_diagnostics(conn, docs, published)?;
        }
        _ => {}
    }
    Ok(())
}

/// Every open file, by path: its URI, version and text. By path, as
/// editors write the same file's URI differently on
/// Windows, `file:///c%3A/..` or `file:///C:/..`, and the server must
/// answer with the one the editor knows the file by.
struct Docs {
    files: HashMap<PathBuf, Document>,
    cache: RefCell<Cache>,
    libraries: LibrarySources,
}

struct Document {
    uri: Uri,
    version: i32,
    text: Rc<String>,
}

#[derive(Default)]
struct Cache {
    /// Analyses shared by every open document in the same program.
    programs: HashMap<PathBuf, Rc<Analysis>>,
    /// Root discovery also depends on absent or non-owning `main.t`s.
    roots: HashMap<PathBuf, (PathBuf, Snapshot)>,
    /// Library buffers use a valid empty program's library information,
    /// independent of any user's program, errors or open-file changes.
    libraries: Option<Rc<Analysis>>,
}

type Snapshot = HashMap<PathBuf, Input>;

/// Everything a read depends on, including failures and file identity:
/// retargeting a symlink can change whether loading detects a cycle.
#[derive(Clone, PartialEq, Eq)]
struct Input {
    text: std::result::Result<Rc<String>, (std::io::ErrorKind, String)>,
    document: Option<(Uri, i32)>,
    canonical: Option<PathBuf>,
}

/// Reads each input once while discovering and checking a snapshot.
struct Inputs<'a> {
    docs: &'a Docs,
    seen: RefCell<Snapshot>,
    reads: RefCell<HashSet<PathBuf>>,
}

impl Inputs<'_> {
    fn input(&self, path: &Path) -> Input {
        let mut seen = self.seen.borrow_mut();
        seen.entry(path.to_owned())
            .or_insert_with(|| self.docs.input(path))
            .clone()
    }

    fn read(&self, path: &Path) -> std::io::Result<String> {
        self.reads.borrow_mut().insert(path.to_owned());
        self.input(path)
            .text
            .map(|text| (*text).clone())
            .map_err(|(kind, message)| std::io::Error::new(kind, message))
    }

    fn matches(&self, snapshot: &Snapshot) -> bool {
        snapshot
            .iter()
            .all(|(path, input)| self.input(path) == *input)
    }

    /// Validation may have inspected obsolete dependencies. Only actual
    /// reads belong in the new discovery or analysis snapshot.
    fn take_snapshot(&self) -> Snapshot {
        let seen = self.seen.borrow();
        self.reads
            .borrow_mut()
            .drain()
            .map(|path| {
                let input = seen[&path].clone();
                (path, input)
            })
            .collect()
    }
}

impl Docs {
    fn new() -> std::io::Result<Self> {
        Ok(Self {
            files: HashMap::new(),
            cache: RefCell::default(),
            libraries: LibrarySources::new()?,
        })
    }

    fn is_library(&self, uri: &Uri) -> bool {
        self.libraries.source(&uri_path(uri)).is_some()
    }

    /// The editor has the file at `uri` open, with `text`.
    fn open(&mut self, uri: &Uri, version: i32, text: String) {
        let path = uri_path(uri);
        // Library copies are for browsing. Neither editor overlays nor disk
        // changes can replace the sources this compiler actually checked.
        let text = self.libraries.source(&path).map_or(text, str::to_owned);
        self.files.insert(
            path,
            Document {
                uri: uri.clone(),
                version,
                text: Rc::new(text),
            },
        );
    }

    /// The text of the open file at `uri`.
    fn text(&self, uri: &Uri) -> Option<&String> {
        self.files.get(&uri_path(uri)).map(|doc| doc.text.as_ref())
    }

    /// The URI of the file at `path`: the editor's, if it's open.
    fn uri(&self, path: &Path) -> Uri {
        if let Some(path) = self.libraries.path(path) {
            return self
                .files
                .get(&path)
                .map_or_else(|| path_uri(&path), |doc| doc.uri.clone());
        }
        match self.files.get(path) {
            Some(doc) => doc.uri.clone(),
            None => path_uri(path),
        }
    }

    fn input(&self, path: &Path) -> Input {
        let (text, document) = match self.files.get(path) {
            Some(doc) => (Ok(doc.text.clone()), Some((doc.uri.clone(), doc.version))),
            None => (
                crate::load::read_file(path)
                    .map(Rc::new)
                    .map_err(|e| (e.kind(), e.to_string())),
                None,
            ),
        };
        Input {
            text,
            document,
            canonical: std::fs::canonicalize(path).ok(),
        }
    }

    /// The program the open file at `uri` is in, checked. That's the one
    /// of the nearest `main.t` above it that has it as a module's file, or
    /// else the file alone (see `load.rs`).
    fn analyse(&self, uri: &Uri) -> Option<Rc<Analysis>> {
        self.text(uri)?;
        if self.is_library(uri) {
            let mut cache = self.cache.borrow_mut();
            return Some(
                cache
                    .libraries
                    .get_or_insert_with(|| {
                        let checked = crate::check_source("fn main() {}\n");
                        Rc::new(Analysis {
                            root: checked.sources.first().path.clone(),
                            uris: checked
                                .sources
                                .files()
                                .iter()
                                .map(|f| self.uri(&f.path))
                                .collect(),
                            checked,
                            syntax_errors: Vec::new(),
                            inputs: Snapshot::new(),
                        })
                    })
                    .clone(),
            );
        }
        let path = uri_path(uri);
        let inputs = Inputs {
            docs: self,
            seen: RefCell::default(),
            reads: RefCell::default(),
        };
        let mut cache = self.cache.borrow_mut();
        if let Some((root, discovery)) = cache.roots.get(&path)
            && inputs.matches(discovery)
            && let Some(a) = cache.programs.get(root)
            && inputs.matches(&a.inputs)
        {
            return Some(a.clone());
        }
        let read = |p: &Path| inputs.read(p);
        let mut dirs = path.parent().into_iter().flat_map(Path::ancestors);
        let root = dirs.find_map(|dir| {
            let main = dir.join("main.t");
            let text = inputs.read(&main).ok()?;
            if main == path {
                return Some(main);
            }
            let (sources, _) = crate::load::load(&main, &text, &read);
            sources.file(&path).map(|_| main)
        });
        let root = root.unwrap_or_else(|| path.clone());
        // The standalone fallback's text is an input as well.
        inputs.read(&root).ok()?;
        cache
            .roots
            .insert(path, (root.clone(), inputs.take_snapshot()));
        if let Some(a) = cache.programs.get(&root)
            && inputs.matches(&a.inputs)
        {
            return Some(a.clone());
        }
        let text = inputs.read(&root).ok()?;
        let checked = crate::check_files(&root, &text, &read);
        // Recover only the file that strict loading rejected. Tokens keep
        // their program offsets, including those in notes and quick fixes.
        // Loading errors and lexical errors retain the original diagnostic.
        let syntax_errors = match &checked.result {
            Err(e) if e.phase == Phase::Syntax => {
                let file = checked.sources.file_at(e.span.start);
                crate::lexer::lex(checked.sources.src(file))
                    .map(|mut tokens| {
                        for token in &mut tokens {
                            token.span.start += file.start;
                            token.span.end += file.start;
                        }
                        crate::parser::parse_for_editor(tokens).errors
                    })
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let uris = checked
            .sources
            .files()
            .iter()
            .map(|f| self.uri(&f.path))
            .collect();
        let a = Rc::new(Analysis {
            root: root.clone(),
            checked,
            syntax_errors,
            uris,
            inputs: inputs.take_snapshot(),
        });
        cache.programs.insert(root, a.clone());
        Some(a)
    }
}

/// What checking the program an open file is in found.
struct Analysis {
    /// The program's first file.
    root: PathBuf,
    checked: crate::Checked,
    /// Syntax diagnostics from editor recovery, never a checked program.
    syntax_errors: Vec<Error>,
    /// The URI of each of its files, in the order of its sources.
    uris: Vec<Uri>,
    inputs: Snapshot,
}

impl Analysis {
    fn sources(&self) -> &Sources {
        &self.checked.sources
    }

    fn info(&self) -> &Info {
        &self.checked.info
    }

    /// The file at `uri`, if it's one of the program's.
    fn file(&self, uri: &Uri) -> Option<&SourceFile> {
        let i = self.uris.iter().position(|u| u == uri).or_else(|| {
            // A library analysis outlives opening further buffers, whose
            // clients may spell the same file URI differently on Windows.
            let path = uri_path(uri);
            self.uris.iter().position(|u| uri_path(u) == path)
        })?;
        Some(&self.sources().files()[i])
    }

    /// Where `pos` in the file at `uri` is in the sources.
    fn offset(&self, uri: &Uri, pos: Position) -> Option<usize> {
        let file = self.file(uri)?;
        Some(file.start + offset(self.sources().src(file), pos))
    }

    /// Where `span` is, as a file and the range in it.
    fn location(&self, span: Span) -> Location {
        let (file, local) = self.sources().local(span);
        let i = self
            .sources()
            .files()
            .iter()
            .position(|f| f.start == file.start);
        let uri = self.uris[i.expect("one of the files")].clone();
        Location::new(uri, lsp_range(self.sources().src(file), local))
    }

    /// Where `span` is, if it belongs to the user's program. Editing and
    /// program references must not include the embedded library copies.
    fn program_location(&self, span: Span) -> Option<Location> {
        (!self.sources().in_library(span.start)).then(|| self.location(span))
    }

    /// The range `span` is in the file at `uri`, if it's in that file.
    fn range_in(&self, uri: &Uri, span: Span) -> Option<Range> {
        let location = self.location(span);
        (location.uri == *uri || uri_path(&location.uri) == uri_path(uri)).then_some(location.range)
    }

    /// `edits` as an LSP workspace edit, each in its own file.
    fn workspace_edit(&self, edits: &[crate::error::Edit]) -> WorkspaceEdit {
        // `WorkspaceEdit` wants a map by `Uri`, which clippy thinks may
        // change under it; it doesn't.
        #[allow(clippy::mutable_key_type)]
        let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
        for e in edits {
            let Some(location) = self.program_location(e.span) else {
                continue;
            };
            let edit = TextEdit::new(location.range, e.text.clone());
            changes.entry(location.uri).or_default().push(edit);
        }
        WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }
    }
}

fn respond(req: Request, docs: &Docs) -> Result<Response> {
    match req.method.as_str() {
        CodeActionRequest::METHOD => {
            let p: CodeActionParams = serde_json::from_value(req.params)?;
            let uri = &p.text_document.uri;
            let actions = match docs.analyse(uri) {
                Some(a) => code_actions(&a, uri, p.range),
                None => Vec::new(),
            };
            Ok(Response::new_ok(req.id, actions))
        }
        HoverRequest::METHOD => {
            let p: HoverParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let uri = &at.text_document.uri;
            let hover = docs.analyse(uri).and_then(|a| {
                let note = a.info().note_at(a.offset(uri, at.position)?)?;
                // The signature, then the doc comment, as rust-analyzer
                // shows them.
                let hover = a.info().traits.shorten(&note.hover());
                let mut value = format!("```t\n{hover}\n```");
                if let Some(doc) = &note.doc {
                    value.push_str(&format!("\n\n---\n\n{doc}"));
                }
                Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value,
                    }),
                    range: a.range_in(uri, note.span),
                })
            });
            Ok(Response::new_ok(req.id, hover))
        }
        GotoDefinition::METHOD => {
            let p: GotoDefinitionParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let uri = &at.text_document.uri;
            let def = docs.analyse(uri).and_then(|a| {
                let offset = a.offset(uri, at.position)?;
                let location = match a.info().note_at(offset) {
                    // A module in a file of its own goes to its file.
                    Some(note) if matches!(note.kind, NoteKind::Module(_)) => {
                        let def = note.def?;
                        file_of_mod(&a, def.start).unwrap_or_else(|| a.location(def))
                    }
                    Some(note) => a.location(note.def?),
                    // The name of a `mod m;` goes to its file.
                    None => file_of_mod(&a, offset)?,
                };
                Some(GotoDefinitionResponse::Scalar(location))
            });
            Ok(Response::new_ok(req.id, def))
        }
        GotoTypeDefinition::METHOD => {
            let p: GotoDefinitionParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let uri = &at.text_document.uri;
            let defs = docs.analyse(uri).and_then(|a| {
                let spans = type_definitions(a.info(), a.offset(uri, at.position)?);
                let locations: Vec<Location> = spans.into_iter().map(|s| a.location(s)).collect();
                (!locations.is_empty()).then_some(GotoDefinitionResponse::Array(locations))
            });
            Ok(Response::new_ok(req.id, defs))
        }
        References::METHOD => {
            let p: ReferenceParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position;
            let uri = &at.text_document.uri;
            let refs = docs.analyse(uri).and_then(|a| {
                let def = a.info().note_at(a.offset(uri, at.position)?)?.def?;
                let spans = a.info().occurrences(def).into_iter();
                let spans = spans.filter(|&s| p.context.include_declaration || s != def);
                Some(
                    spans
                        .filter_map(|span| a.program_location(span))
                        .collect::<Vec<_>>(),
                )
            });
            Ok(Response::new_ok(req.id, refs))
        }
        DocumentHighlightRequest::METHOD => {
            let p: DocumentHighlightParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let uri = &at.text_document.uri;
            let highlights = docs.analyse(uri).and_then(|a| {
                let def = a.info().note_at(a.offset(uri, at.position)?)?.def?;
                // Only those in this file: highlights are for one.
                let highlights = a.info().occurrences(def).into_iter().filter_map(|span| {
                    Some(DocumentHighlight {
                        range: a.range_in(uri, span)?,
                        kind: Some(match span == def {
                            true => DocumentHighlightKind::WRITE,
                            false => DocumentHighlightKind::READ,
                        }),
                    })
                });
                Some(highlights.collect::<Vec<_>>())
            });
            Ok(Response::new_ok(req.id, highlights))
        }
        PrepareRenameRequest::METHOD => {
            let at: TextDocumentPositionParams = serde_json::from_value(req.params)?;
            let uri = &at.text_document.uri;
            let Some(a) = docs.analyse(uri) else {
                return Ok(Response::new_ok(req.id, ()));
            };
            let target = renaming(&a).and_then(|(sources, prog, info)| {
                let offset = a
                    .offset(uri, at.position)
                    .ok_or("this file isn't in the program")?;
                let note = crate::rename::target(sources, prog, info, offset)?;
                a.range_in(uri, note.span)
                    .ok_or_else(|| "it's in another file".to_string())
            });
            Ok(match target {
                Ok(range) => Response::new_ok(req.id, PrepareRenameResponse::Range(range)),
                Err(why) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, why),
            })
        }
        Rename::METHOD => {
            let p: RenameParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position;
            let uri = &at.text_document.uri;
            let Some(a) = docs.analyse(uri) else {
                return Ok(Response::new_ok(req.id, ()));
            };
            let edits = renaming(&a).and_then(|(sources, prog, info)| {
                let offset = a
                    .offset(uri, at.position)
                    .ok_or("this file isn't in the program")?;
                let note = crate::rename::target(sources, prog, info, offset)?;
                let def = note.def.expect("a target is defined somewhere");
                crate::rename::rename(sources, prog, info, def, &p.new_name)
            });
            Ok(match edits {
                Ok(edits) => Response::new_ok(req.id, a.workspace_edit(&edits)),
                Err(why) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, why),
            })
        }
        DocumentSymbolRequest::METHOD => {
            let p: DocumentSymbolParams = serde_json::from_value(req.params)?;
            let outline = docs.text(&p.text_document.uri).and_then(|src| {
                let parsed = crate::parser::parse_for_editor(crate::lexer::lex(src).ok()?);
                Some(DocumentSymbolResponse::Nested(outline(
                    src,
                    &parsed.program,
                )))
            });
            Ok(Response::new_ok(req.id, outline))
        }
        InlayHintRequest::METHOD => {
            let p: InlayHintParams = serde_json::from_value(req.params)?;
            let uri = &p.text_document.uri;
            let hints = match docs.analyse(uri) {
                Some(a) => inlay_hints(&a, uri, p.range),
                None => Vec::new(),
            };
            Ok(Response::new_ok(req.id, hints))
        }
        Formatting::METHOD => {
            let p: DocumentFormattingParams = serde_json::from_value(req.params)?;
            if docs.is_library(&p.text_document.uri) {
                return Ok(Response::new_ok(req.id, ()));
            }
            let Some(src) = docs.text(&p.text_document.uri) else {
                return Ok(Response::new_ok(req.id, ()));
            };
            match crate::fmt::format(src) {
                // One edit replacing the whole file: simpler than a diff, and
                // editors keep the cursor where it was well enough.
                Ok(formatted) => {
                    let edits = match formatted == *src {
                        true => Vec::new(),
                        false => vec![TextEdit::new(
                            Range::new(Position::new(0, 0), position(src, src.len())),
                            formatted,
                        )],
                    };
                    Ok(Response::new_ok(req.id, edits))
                }
                // The syntax error is already showing as a diagnostic.
                Err(FmtError::Syntax(_)) => Ok(Response::new_ok(req.id, ())),
                Err(FmtError::Changed(why)) => Ok(Response::new_err(
                    req.id,
                    ErrorCode::RequestFailed as i32,
                    format!("not formatting: {why}"),
                )),
            }
        }
        _ => {
            let msg = format!("unsupported request `{}`", req.method);
            Ok(Response::new_err(
                req.id,
                ErrorCode::MethodNotFound as i32,
                msg,
            ))
        }
    }
}

/// Renaming uses the same checked AST as the other semantic requests.
/// The proposed edits are still checked separately by `rename::rename`.
fn renaming(a: &Analysis) -> std::result::Result<(&Sources, &Program, &Info), String> {
    let program = a.checked.result.as_ref().map_err(|e| {
        let kind = match e.phase {
            Phase::Syntax => "syntax",
            _ => "type",
        };
        format!("the program has a {kind} error; fix it first")
    })?;
    Ok((a.sources(), program.program(), program.info()))
}

/// The file of the `mod m;` whose name is at `offset`, if there's one.
fn file_of_mod(a: &Analysis, offset: usize) -> Option<Location> {
    let files = a.sources().files();
    let i = files
        .iter()
        .position(|f| f.declared.is_some_and(|name| name.contains(offset)))?;
    Some(Location::new(a.uris[i].clone(), Range::default()))
}

fn publish(
    conn: &Connection,
    uri: Uri,
    version: Option<i32>,
    diagnostics: Vec<Diagnostic>,
) -> Result<()> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version,
    };
    conn.sender
        .send(Notification::new(PublishDiagnostics::METHOD.to_string(), params).into())?;
    Ok(())
}

/// Reconcile every open file's current program before clearing diagnostics.
/// An edit can detach an open module or attach a previously standalone one;
/// publishing one root at a time would clear another program's diagnostics.
fn refresh_diagnostics(
    conn: &Connection,
    docs: &Docs,
    published: &mut HashMap<PathBuf, Rc<Analysis>>,
) -> Result<()> {
    let mut current = HashMap::new();
    let mut paths: Vec<_> = docs.files.keys().collect();
    paths.sort();
    for path in paths {
        if docs.libraries.source(path).is_some() {
            continue;
        }
        if let Some(a) = docs.analyse(&docs.files[path].uri) {
            current.insert(a.root.clone(), a);
        }
    }
    if current.len() == published.len()
        && current
            .iter()
            .all(|(root, a)| published.get(root).is_some_and(|old| Rc::ptr_eq(old, a)))
    {
        return Ok(());
    }
    let before: HashMap<_, _> = published
        .values()
        .flat_map(|a| {
            a.sources()
                .program_files()
                .iter()
                .map(|f| (f.path.clone(), a.location(f.span()).uri))
        })
        .collect();
    let mut affected = HashSet::new();
    for (root, a) in &current {
        if !published.get(root).is_some_and(|old| Rc::ptr_eq(old, a)) {
            affected.extend(a.sources().program_files().iter().map(|f| f.path.clone()));
            if let Some(old) = published.get(root) {
                affected.extend(old.sources().program_files().iter().map(|f| f.path.clone()));
            }
        }
    }
    for (root, old) in published.iter() {
        if !current.contains_key(root) {
            affected.extend(old.sources().program_files().iter().map(|f| f.path.clone()));
        }
    }
    let mut affected: Vec<_> = affected.into_iter().collect();
    affected.sort();
    let mut programs: Vec<_> = current.values().collect();
    programs.sort_by_key(|a| &a.root);
    for path in affected {
        let uri = docs.uri(&path);
        let mut live = false;
        let mut diagnostics = Vec::new();
        // Two open roots can load the same module file. Publish all its
        // diagnostics together, including those from an unchanged root.
        for a in &programs {
            if a.sources().file(&path).is_some() {
                live = true;
                for (d, _) in diagnose(a, &uri) {
                    if !diagnostics.contains(&d) {
                        diagnostics.push(d);
                    }
                }
            }
        }
        let doc = docs.files.get(&path);
        if live {
            if !diagnostics.is_empty() || doc.is_some() || before.contains_key(&path) {
                publish(conn, uri, doc.map(|doc| doc.version), diagnostics)?;
            }
        } else if let Some(old_uri) = before.get(&path) {
            publish(conn, old_uri.clone(), None, Vec::new())?;
        }
    }
    let mut cache = docs.cache.borrow_mut();
    cache.roots.retain(|path, _| docs.files.contains_key(path));
    cache.programs.retain(|root, _| current.contains_key(root));
    *published = current;
    Ok(())
}

/// Every diagnostic in the file at `uri` of the program `a` is about, each
/// with its fixes.
fn diagnose(a: &Analysis, uri: &Uri) -> Vec<(Diagnostic, Vec<Fix>)> {
    let mut out: Vec<_> = a
        .checked
        .warnings
        .iter()
        .filter_map(|w| Some((warning_diagnostic(a, uri, w)?, w.fixes.clone())))
        .collect();
    let errors: &[_] = match &a.checked.result {
        Err(e) if a.syntax_errors.is_empty() => std::slice::from_ref(e),
        _ => &a.syntax_errors,
    };
    for e in errors {
        if let Some(d) = error_diagnostic(a, uri, e) {
            out.push((d, e.fixes.clone()));
        }
    }
    out
}

/// A quick fix for each fix of each diagnostic in the file at `uri` that
/// touches `range`.
fn code_actions(a: &Analysis, uri: &Uri, range: Range) -> Vec<CodeActionOrCommand> {
    if a.file(uri).is_some_and(SourceFile::is_library) {
        return Vec::new();
    }
    let mut actions = Vec::new();
    for (diag, fixes) in diagnose(a, uri) {
        if diag.range.start > range.end || range.start > diag.range.end {
            continue;
        }
        // The fix to apply without asking, if the editor does that, is the
        // only one there is.
        let preferred = fixes.len() == 1;
        for fix in fixes {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                edit: Some(a.workspace_edit(&fix.edits)),
                title: fix.title,
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                is_preferred: Some(preferred),
                ..Default::default()
            }));
        }
    }
    // Writing what's inferred of a function's effects, on its name.
    for (span, fix) in &a.info().inferred_fixes {
        let Some(at) = a.range_in(uri, *span) else {
            continue;
        };
        if at.start > range.end || range.start > at.end {
            continue;
        }
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            edit: Some(a.workspace_edit(&fix.edits)),
            title: fix.title.clone(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            ..Default::default()
        }));
    }
    actions
}

/// Where the structs and enums in the type of the variable, field or
/// function at `offset` are declared, in the order the type mentions them.
fn type_definitions(info: &Info, offset: usize) -> Vec<Span> {
    let Some(note) = info.note_at(offset) else {
        return Vec::new();
    };
    let (NoteKind::Var(_) | NoteKind::Field(_) | NoteKind::Fn(_)) = note.kind else {
        return Vec::new();
    };
    let mut symbols = Vec::new();
    if let Some(ty) = &note.ty {
        adts(ty, &mut symbols);
    }
    let mut out: Vec<Span> = Vec::new();
    for symbol in symbols {
        if let Some(&span) = info.types.get(symbol)
            && !out.contains(&span)
        {
            out.push(span);
        }
    }
    out
}

/// Adds the symbol of every struct and enum `t` mentions to `out`, in order.
fn adts<'t>(t: &'t Type, out: &mut Vec<&'t str>) {
    if let Type::Adt(symbol, ..) = t {
        out.push(symbol);
    }
    for p in t.parts() {
        adts(p, out);
    }
}

/// The outline of `prog`: its modules, functions, structs, enums, traits
/// and impls, with the items of each module, the fields and variants of
/// each type, the methods of each trait and impl, and the nested functions
/// and local types of each function, under it. In source order.
fn outline(src: &str, prog: &Program) -> Vec<DocumentSymbol> {
    let fns = prog.all_fns();
    let mut inside: Vec<Vec<DocumentSymbol>> = vec![Vec::new(); fns.len()];
    let mut impls: Vec<Vec<DocumentSymbol>> = vec![Vec::new(); prog.impls.len()];
    let mut traits: Vec<Vec<DocumentSymbol>> = vec![Vec::new(); prog.traits.len()];
    // Those with a default are among the functions.
    for (t, children) in prog.traits.iter().zip(&mut traits) {
        for m in t.methods.iter().filter(|m| !m.provided) {
            let mut s = fn_symbol(src, &m.decl, Vec::new());
            s.kind = SymbolKind::METHOD;
            children.push(s);
        }
    }
    // The items of each module.
    let mut top: Vec<Vec<DocumentSymbol>> = vec![Vec::new(); prog.mods.len()];
    for (d, f) in prog.all_types() {
        let s = type_symbol(src, d);
        match f {
            Some(f) => inside[f].push(s),
            None => top[d.module()].push(s),
        }
    }
    // Nested functions come after the ones they're in.
    for (i, &(f, parent, method)) in fns.iter().enumerate().rev() {
        let mut children = std::mem::take(&mut inside[i]);
        sort(&mut children);
        let mut s = fn_symbol(src, f, children);
        match (parent, method) {
            (Some(p), _) => inside[p].push(s),
            (None, Some(of)) => {
                s.kind = SymbolKind::METHOD;
                match of {
                    MethodOf::Impl(k) => impls[k].push(s),
                    MethodOf::Trait(k) => traits[k].push(s),
                }
            }
            (None, None) => top[f.module].push(s),
        }
    }
    for (t, mut children) in prog.traits.iter().zip(traits) {
        sort(&mut children);
        let detail = format!("trait {}", t.name.name);
        top[t.module].push(symbol(
            src,
            label(&t.name),
            detail,
            SymbolKind::INTERFACE,
            t.span,
            children,
        ));
    }
    for (d, mut children) in prog.impls.iter().zip(impls) {
        sort(&mut children);
        let header = match &d.trait_name {
            Some(t) => format!("impl {} for {}", t.name, d.self_ty),
            None => format!("impl {}", d.self_ty),
        };
        top[d.module].push(symbol(
            src,
            (&header.clone(), d.self_ty.span),
            header,
            SymbolKind::OBJECT,
            d.span,
            children,
        ));
    }
    // A module comes after the one it's in.
    for (i, m) in prog.mods.iter().enumerate().skip(1).rev() {
        let mut children = std::mem::take(&mut top[i]);
        sort(&mut children);
        let detail = format!("mod {}", m.name.name);
        let s = symbol(
            src,
            label(&m.name),
            detail,
            SymbolKind::MODULE,
            m.span,
            children,
        );
        top[m.parent.expect("not the crate root")].push(s);
    }
    let mut top = std::mem::take(&mut top[ROOT]);
    sort(&mut top);
    top
}

fn sort(symbols: &mut [DocumentSymbol]) {
    symbols.sort_by_key(|s| (s.range.start.line, s.range.start.character));
}

/// A symbol for the outline. `detail` is the signature, as hover shows it.
fn symbol(
    src: &str,
    (name, at): (&str, Span),
    detail: String,
    kind: SymbolKind,
    span: Span,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: name.to_string(),
        detail: Some(detail),
        kind,
        tags: None,
        deprecated: None,
        range: lsp_range(src, span),
        selection_range: lsp_range(src, at),
        children: Some(children),
    }
}

/// A name, as `symbol` takes it: what it says, and where.
fn label(name: &crate::ast::Ident) -> (&str, Span) {
    (&name.name, name.span)
}

fn fn_symbol(src: &str, f: &FnDecl, children: Vec<DocumentSymbol>) -> DocumentSymbol {
    let params: Vec<&crate::ast::TypeExpr> = f.params.iter().map(|p| &p.ty).collect();
    let detail = format!("fn({}) -> {}", comma_list(&params), f.ret);
    symbol(
        src,
        label(&f.name),
        detail,
        SymbolKind::FUNCTION,
        f.span,
        children,
    )
}

fn type_symbol(src: &str, d: &TypeDecl) -> DocumentSymbol {
    let name = &d.name().name;
    let ty = match d.type_params() {
        [] => name.clone(),
        params => {
            let params: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            format!("{name}<{}>", params.join(", "))
        }
    };
    let (keyword, kind, children) = match d {
        TypeDecl::Struct(s) => {
            let fields = s.fields.iter().map(|f| {
                let span = f.name.span.to(f.ty.span);
                symbol(
                    src,
                    label(&f.name),
                    f.ty.to_string(),
                    SymbolKind::FIELD,
                    span,
                    Vec::new(),
                )
            });
            ("struct", SymbolKind::STRUCT, fields.collect())
        }
        TypeDecl::Enum(e) => {
            let variants = e.variants.iter().map(|v| {
                let detail = match v.fields.as_slice() {
                    [] => ty.clone(),
                    fields => format!("fn({}) -> {ty}", comma_list(fields)),
                };
                symbol(
                    src,
                    label(&v.name),
                    detail,
                    SymbolKind::ENUM_MEMBER,
                    v.span,
                    Vec::new(),
                )
            });
            ("enum", SymbolKind::ENUM, variants.collect())
        }
        TypeDecl::Alias(a) => {
            let detail = format!("type {ty} = {}", a.ty);
            let kind = SymbolKind::TYPE_PARAMETER;
            return symbol(src, label(d.name()), detail, kind, d.span(), Vec::new());
        }
    };
    let detail = format!("{keyword} {ty}");
    symbol(src, label(d.name()), detail, kind, d.span(), children)
}

/// The types the checker inferred, and the type arguments it worked out for
/// each use of something generic, inside `range` of the file at `uri`.
fn inlay_hints(a: &Analysis, uri: &Uri, range: Range) -> Vec<InlayHint> {
    let mut notes: Vec<_> = a.info().notes.iter().collect();
    notes.sort_by_key(|n| n.span.start);
    notes
        .iter()
        .filter_map(|n| {
            let hint = a.info().traits.shorten(&n.hint()?);
            let end = Span::new(n.span.end, n.span.end);
            let pos = a.range_in(uri, end)?.start;
            (range.start <= pos && pos <= range.end).then(|| InlayHint {
                position: pos,
                label: InlayHintLabel::String(hint),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: None,
                padding_left: None,
                padding_right: None,
                data: None,
            })
        })
        .collect()
}

/// The diagnostic for `e`, if it's in the file at `uri`. Its notes, like
/// "`x` is borrowed here", are related information, in any file.
fn error_diagnostic(a: &Analysis, uri: &Uri, e: &Error) -> Option<Diagnostic> {
    let phase = match e.phase {
        Phase::Syntax => "syntax error",
        Phase::Type => "type error",
        Phase::Runtime => "runtime error",
    };
    // Same-site explanations belong in the hover text too; clients often
    // display related information only as a list of other locations.
    let mut message = format!("{phase}: {}", e.message);
    for (_, note) in e.notes.iter().filter(|(span, _)| *span == e.span) {
        message.push_str(&format!("\n{note}"));
    }
    let related = e
        .notes
        .iter()
        .filter(|(span, _)| *span != e.span)
        .map(|(span, note)| DiagnosticRelatedInformation {
            location: a.location(*span),
            message: note.clone(),
        })
        .collect::<Vec<_>>();
    Some(Diagnostic {
        range: a.range_in(uri, e.span)?,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("t".to_string()),
        message,
        related_information: (!related.is_empty()).then_some(related),
        ..Default::default()
    })
}

fn warning_diagnostic(a: &Analysis, uri: &Uri, w: &Warning) -> Option<Diagnostic> {
    Some(Diagnostic {
        range: a.range_in(uri, w.span)?,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(w.lint.name().to_string())),
        source: Some("t".to_string()),
        message: w.message.clone(),
        // Editors fade out code tagged unnecessary, which is just right for
        // code that can never run and variables nothing reads.
        tags: matches!(
            w.lint,
            Lint::UnreachablePatterns | Lint::UnusedVariables | Lint::DeadCode
        )
        .then(|| vec![DiagnosticTag::UNNECESSARY]),
        ..Default::default()
    })
}

/// The path of a `file:` URI. Any other URI, of a file not saved, is a
/// path of its own, which no `main.t` is above.
fn uri_path(uri: &Uri) -> PathBuf {
    PathBuf::from(file_path(uri.as_str(), cfg!(windows)))
}

/// The `file:` URI of a path, the inverse of `uri_path`.
fn path_uri(path: &Path) -> Uri {
    let s = path.to_string_lossy();
    // Not a path, but the URI of a file not saved, as `uri_path` gives.
    if !path.has_root()
        && let Ok(uri) = s.parse()
    {
        return uri;
    }
    file_uri(&s, cfg!(windows)).parse().expect("a file URI")
}

/// The path `uri` names, on Windows if `windows`: `file:///home/a.t` is
/// `/home/a.t`; on Windows, `file:///c%3A/a.t` is `C:\a.t`, with its drive
/// letter in capitals, as editors differ, and `file://server/share/a.t` is
/// `\\server\share\a.t`. A URI that isn't a `file:` one is its own path.
fn file_path(uri: &str, windows: bool) -> String {
    let Some(rest) = uri.strip_prefix("file://") else {
        return uri.to_string();
    };
    let bytes = rest.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (
            bytes[i],
            bytes.get(i + 1).copied().and_then(hex),
            bytes.get(i + 2).copied().and_then(hex),
        ) {
            (b'%', Some(hi), Some(lo)) => {
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            (b, ..) => {
                out.push(b);
                i += 1;
            }
        }
    }
    let path = String::from_utf8_lossy(&out).into_owned();
    if !windows {
        return path;
    }
    let path = match path.strip_prefix('/') {
        Some(local) if drive(local) => local[..1].to_ascii_uppercase() + &local[1..],
        Some(_) => path,
        // A host: a share on another machine.
        None => format!("//{path}"),
    };
    path.replace('/', "\\")
}

/// The `file:` URI of `path`, on Windows if `windows`: the inverse of
/// `file_path`. Each byte but the few a URI path may have as they are is
/// escaped, `%20` for a space, and so is `\` elsewhere than on Windows,
/// where it separates the parts of a path.
fn file_uri(path: &str, windows: bool) -> String {
    let path = match windows {
        true => path.replace('\\', "/"),
        false => path.to_string(),
    };
    let (start, path) = match path.strip_prefix("//") {
        Some(share) if windows => ("file://", share.to_string()),
        _ if windows && drive(&path) => ("file:///", path),
        _ => ("file://", path),
    };
    let mut out = String::from(start);
    for (i, b) in path.bytes().enumerate() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            // A drive's `C:` as it is, as RFC 8089 and Neovim write it.
            b':' if windows && i == 1 && start == "file:///" => out.push(':'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Whether `path` starts with a Windows drive, `C:`.
fn drive(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b.len() == 2 || b[2] == b'/')
}

fn lsp_range(src: &str, span: Span) -> Range {
    Range::new(position(src, span.start), position(src, span.end))
}

/// Converts a byte offset into an LSP position. LSP columns count UTF-16 code
/// units by default (a JavaScript heritage), so `𝕏` is 2 columns, not 1 or 4.
fn position(src: &str, offset: usize) -> Position {
    let offset = offset.min(src.len());
    let line_start = src[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line = src[..offset].matches('\n').count();
    let character: usize = src[line_start..offset].chars().map(char::len_utf16).sum();
    Position::new(line as u32, character as u32)
}

/// Converts an LSP position back into a byte offset: the inverse of
/// `position`. One past the end of a line is the end of that line.
fn offset(src: &str, pos: Position) -> usize {
    let line_start: usize = src
        .split_inclusive('\n')
        .take(pos.line as usize)
        .map(str::len)
        .sum();
    let mut units = 0;
    for (i, c) in src[line_start..].char_indices() {
        if units >= pos.character as usize || c == '\n' {
            return line_start + i;
        }
        units += c.len_utf16();
    }
    src.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_copies_preserve_every_embedded_byte_and_have_separate_identities() {
        let a = LibrarySources::new().unwrap();
        let b = LibrarySources::new().unwrap();
        for (name, files, dir) in crate::load::LIBRARIES {
            for (file, text) in files {
                let source = Path::new(dir).join(file);
                let path = a.path(&source).unwrap();
                assert!(path.ends_with(Path::new(name).join(file)));
                assert_eq!(std::fs::read(&path).unwrap(), text.as_bytes());
                assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
                assert_eq!(a.source(&path), Some(*text));
                assert_ne!(b.path(&source).unwrap(), path);
                assert_eq!(b.source(&path), None);
            }
        }
        assert_eq!(a.path(Path::new("core/option.t")), None);
        assert_eq!(a.path(Path::new("<core>/unknown.t")), None);
    }

    #[test]
    fn library_buffers_share_analysis_without_invalidating_program_snapshots() {
        let mut docs = Docs::new().unwrap();
        let main = path_uri(&std::env::temp_dir().join("t-library-cache-main.t"));
        docs.open(&main, 1, "fn main() {}\n".into());
        let program = docs.analyse(&main).unwrap();
        let uri = docs.uri(Path::new("<core>/option.t"));
        docs.open(&uri, 1, "fn broken(".into());
        let library = docs.analyse(&uri).unwrap();
        assert!(library.checked.result.is_ok());
        let file = library.file(&uri).unwrap();
        assert!(file.is_library());
        assert_eq!(docs.text(&uri).unwrap(), library.sources().src(file));
        assert!(Rc::ptr_eq(&program, &docs.analyse(&main).unwrap()));
        let alloc = docs.uri(Path::new("<alloc>/list.t"));
        docs.open(&alloc, 1, String::new());
        assert!(Rc::ptr_eq(&library, &docs.analyse(&alloc).unwrap()));
        docs.open(&main, 2, "fn main( {".into());
        assert!(docs.analyse(&main).unwrap().checked.result.is_err());
        docs.open(&uri, 2, String::new());
        assert!(Rc::ptr_eq(&library, &docs.analyse(&uri).unwrap()));
        // Even a proposed quick fix must never edit library sources.
        let edits = [crate::error::Edit::insert(file.start, "changed")];
        assert!(library.workspace_edit(&edits).changes.unwrap().is_empty());
    }

    #[test]
    fn semantic_requests_share_analysis_until_an_input_changes() {
        let dir = std::env::temp_dir().join(format!("t-lsp-snapshot-{}", std::process::id()));
        let main = path_uri(&dir.join("main.t"));
        let util = path_uri(&dir.join("util.t"));
        let src = "mod util;\nfn main() {\n    fn nested() -> i64 { 1 }\n    let _n: i64 = util::id(nested());\n}\n";
        let helper = "pub fn id<T: Copy>(x: T) -> T { x }\n";
        let mut docs = Docs::new().unwrap();
        docs.open(&main, 1, src.into());
        docs.open(&util, 1, helper.into());
        let a = docs.analyse(&util).unwrap();
        let checked = a.checked.result.as_ref().unwrap();
        // Analysis retains generics and nesting: no executable lowering.
        assert!(checked.program().fns.iter().any(|f| !f.generics.is_empty()));
        assert!(
            !checked
                .program()
                .fns
                .iter()
                .any(|f| f.name.name == "nested")
        );
        assert!(Rc::ptr_eq(&a, &docs.analyse(&main).unwrap()));

        for (method, extra) in [
            (
                HoverRequest::METHOD,
                serde_json::json!({"position": {"line": 3, "character": 24}}),
            ),
            (
                References::METHOD,
                serde_json::json!({"position": {"line": 3, "character": 24}, "context": {"includeDeclaration": true}}),
            ),
            (
                InlayHintRequest::METHOD,
                serde_json::json!({"range": {"start": {"line": 0, "character": 0}, "end": {"line": 5, "character": 0}}}),
            ),
            (
                PrepareRenameRequest::METHOD,
                serde_json::json!({"position": {"line": 3, "character": 24}}),
            ),
        ] {
            let mut params = extra;
            params["textDocument"] = serde_json::json!({"uri": main});
            let response = respond(Request::new(1.into(), method.into(), params), &docs).unwrap();
            assert!(response.response_result.is_ok(), "{response:?}");
            assert!(Rc::ptr_eq(&a, &docs.analyse(&main).unwrap()));
        }

        // Opening an unrelated overlay doesn't invalidate this program.
        docs.open(&path_uri(&dir.join("other.t")), 1, "fn main() {}".into());
        assert!(Rc::ptr_eq(&a, &docs.analyse(&main).unwrap()));
        // Versions are part of a snapshot, even if the text is unchanged.
        docs.open(&util, 2, helper.into());
        let b = docs.analyse(&main).unwrap();
        assert!(!Rc::ptr_eq(&a, &b));
        assert!(Rc::ptr_eq(&b, &docs.analyse(&util).unwrap()));

        docs.open(&util, 3, "pub fn id(_x: i64) -> bool { true }\n".into());
        let failed = docs.analyse(&main).unwrap();
        assert!(failed.checked.result.is_err());
        assert!(!Rc::ptr_eq(&b, &failed));
        assert!(Rc::ptr_eq(&failed, &docs.analyse(&util).unwrap()));

        // Removed modules must leave both the program's snapshot and its
        // root-discovery inputs, even if validation read them first.
        docs.open(&main, 2, "fn main() {}\n".into());
        let detached = docs.analyse(&main).unwrap();
        assert!(detached.checked.result.is_ok());
        docs.open(&util, 4, helper.into());
        assert!(Rc::ptr_eq(&detached, &docs.analyse(&main).unwrap()));
    }

    #[test]
    fn missing_inputs_and_closed_overlays_invalidate_failed_analysis() {
        let dir = std::env::temp_dir().join(format!("t-lsp-missing-{}", std::process::id()));
        let main = path_uri(&dir.join("main.t"));
        let util = path_uri(&dir.join("util.t"));
        let mut docs = Docs::new().unwrap();
        docs.open(&main, 1, "mod util;\nfn main() { util::f(); }\n".into());
        let missing = docs.analyse(&main).unwrap();
        assert!(missing.checked.result.is_err());
        assert!(Rc::ptr_eq(&missing, &docs.analyse(&main).unwrap()));
        docs.open(&util, 1, "pub fn f() {}\n".into());
        let restored = docs.analyse(&main).unwrap();
        assert!(restored.checked.result.is_ok());
        docs.files.remove(&uri_path(&util));
        let closed = docs.analyse(&main).unwrap();
        assert!(closed.checked.result.is_err());
        assert!(!Rc::ptr_eq(&restored, &closed));
    }

    #[test]
    fn position_counts_utf16_units() {
        let src = "ab\né𝕏x\n";
        let x = src.find('x').unwrap();
        // `é` is 1 unit (2 bytes in UTF-8), `𝕏` is 2 units (4 bytes).
        assert_eq!(position(src, x), Position::new(1, 3));
        assert_eq!(position(src, 0), Position::new(0, 0));
        assert_eq!(position(src, src.len()), Position::new(2, 0));
    }

    #[test]
    fn file_uris_are_paths_on_unix() {
        assert_eq!(file_path("file:///home/a%20b/m.t", false), "/home/a b/m.t");
        assert_eq!(file_uri("/home/a b/m.t", false), "file:///home/a%20b/m.t");
        assert_eq!(
            file_path("untitled:Untitled-1", false),
            "untitled:Untitled-1"
        );
    }

    /// Editors write a Windows path's URI differently, VS Code
    /// `file:///c%3A/..` and Neovim `file:///C:/..`: both are the same path.
    #[test]
    fn file_uris_are_paths_on_windows() {
        let path = r"C:\Users\me\t lang\main.t";
        assert_eq!(
            file_path("file:///c%3A/Users/me/t%20lang/main.t", true),
            path
        );
        assert_eq!(file_path("file:///C:/Users/me/t%20lang/main.t", true), path);
        assert_eq!(file_uri(path, true), "file:///C:/Users/me/t%20lang/main.t");
        assert_eq!(
            file_path("file://server/share/m.t", true),
            r"\\server\share\m.t"
        );
        assert_eq!(
            file_uri(r"\\server\share\m.t", true),
            "file://server/share/m.t"
        );
        // A path with no drive, as `file:///tmp/a.t` is, is on the current one.
        assert_eq!(file_path("file:///tmp/a.t", true), r"\tmp\a.t");
        assert_eq!(file_uri(r"\tmp\a.t", true), "file:///tmp/a.t");
        for uri in [
            "file:///C:/a.t",
            "file://server/share/m.t",
            "file:///tmp/a.t",
        ] {
            assert_eq!(file_uri(&file_path(uri, true), true), uri);
        }
    }

    #[test]
    fn offset_inverts_position() {
        let src = "ab\né𝕏x\n";
        for (i, _) in src.char_indices().chain([(src.len(), ' ')]) {
            assert_eq!(offset(src, position(src, i)), i);
        }
    }
}
