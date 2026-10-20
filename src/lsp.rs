//! A Language Server Protocol server, so any LSP-capable editor can show T's
//! errors and warnings as you type, and offer their fixes as quick fixes. Run
//! as `t lsp`; it talks JSON-RPC over stdin/stdout.
//!
//! A file is checked as part of its program: the one of the nearest
//! `main.t` above it that has it as a module's file, or else the file
//! alone (see `load.rs`), with what the editor has for open files and
//! what's on disk for the others. On every open/change it re-checks the
//! whole program (it's tiny and fast) and reports every warning and the
//! first error, each in its file. Asked for anything else, it checks the
//! program again and answers from what the checker found out
//! (`check::Info`): the fixes of the diagnostics in range as code actions,
//! the type and doc comment of what's under the cursor on hover, where
//! that's defined and where its type is, every place it's named (as
//! references and highlights), and inferred types and type arguments as
//! inlay hints. It renames as `rename.rs` does. The outline of a file comes
//! from parsing it alone, so it's there even when the file doesn't check.
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
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
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
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn run() -> Result<()> {
    let (conn, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        // FULL: the editor sends the whole file on every change, not diffs.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
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

    let mut docs = Docs::default();
    // The files each program last had diagnostics published for, by the
    // path of its first file, to clear those of a file no longer in it.
    let mut published: HashMap<PathBuf, Vec<Uri>> = HashMap::new();
    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    break;
                }
                conn.sender.send(respond(req, &docs)?.into())?;
            }
            Message::Notification(n) => match n.method.as_str() {
                DidOpenTextDocument::METHOD => {
                    let p: DidOpenTextDocumentParams = serde_json::from_value(n.params)?;
                    let doc = p.text_document;
                    docs.0.insert(doc.uri.as_str().to_string(), doc.text);
                    let version = (doc.uri, doc.version);
                    publish_all(&conn, &docs, version, &mut published)?;
                }
                DidChangeTextDocument::METHOD => {
                    let p: DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
                    // With FULL sync, the last change holds the whole new text.
                    if let Some(change) = p.content_changes.into_iter().last() {
                        let doc = p.text_document;
                        docs.0.insert(doc.uri.as_str().to_string(), change.text);
                        let version = (doc.uri, doc.version);
                        publish_all(&conn, &docs, version, &mut published)?;
                    }
                }
                DidCloseTextDocument::METHOD => {
                    let p: DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
                    docs.0.remove(p.text_document.uri.as_str());
                    publish(&conn, p.text_document.uri, None, Vec::new())?;
                }
                _ => {}
            },
            Message::Response(_) => {}
        }
    }

    // The writer thread exits once every sender is gone, so `conn` must be
    // dropped before joining or this waits forever.
    drop(conn);
    io_threads.join()?;
    Ok(())
}

/// The text of every open file, by URI.
#[derive(Default)]
struct Docs(HashMap<String, String>);

impl Docs {
    /// Reads a file: what the editor has, if it's open, or else what's on
    /// disk.
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.0.get(path_uri(path).as_str()) {
            Some(text) => Ok(text.clone()),
            None => crate::load::read_file(path),
        }
    }

    /// The program the open file at `uri` is in, checked. That's the one
    /// of the nearest `main.t` above it that has it as a module's file, or
    /// else the file alone (see `load.rs`).
    fn analyse(&self, uri: &Uri) -> Option<Analysis> {
        let src = self.0.get(uri.as_str())?;
        let path = uri_path(uri);
        let read = |p: &Path| self.read(p);
        let mut dirs = path.parent().into_iter().flat_map(Path::ancestors);
        let root = dirs.find_map(|dir| {
            let main = dir.join("main.t");
            if main == path {
                return Some((main, src.clone()));
            }
            let text = self.read(&main).ok()?;
            let (sources, _) = crate::load::load(&main, &text, &read);
            sources.file(&path).map(|_| (main, text))
        });
        let (root, text) = root.unwrap_or_else(|| (path, src.clone()));
        let checked = crate::check_files(&root, &text, &read);
        let uris = checked
            .sources
            .files()
            .iter()
            .map(|f| path_uri(&f.path))
            .collect();
        Some(Analysis {
            root,
            text,
            checked,
            uris,
        })
    }
}

/// What checking the program an open file is in found.
struct Analysis {
    /// The program's first file, and its text.
    root: PathBuf,
    text: String,
    checked: crate::Checked,
    /// The URI of each of its files, in the order of its sources.
    uris: Vec<Uri>,
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
        let i = self.uris.iter().position(|u| u == uri)?;
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

    /// Where `span` is, unless it's in the core library, whose files are
    /// built into the compiler, and so aren't anywhere an editor can open.
    fn program_location(&self, span: Span) -> Option<Location> {
        (!self.sources().in_core(span.start)).then(|| self.location(span))
    }

    /// The range `span` is in the file at `uri`, if it's in that file.
    fn range_in(&self, uri: &Uri, span: Span) -> Option<Range> {
        let location = self.location(span);
        (location.uri == *uri).then_some(location.range)
    }

    /// `edits` as an LSP workspace edit, each in its own file.
    fn workspace_edit(&self, edits: &[crate::error::Edit]) -> WorkspaceEdit {
        // `WorkspaceEdit` wants a map by `Uri`, which clippy thinks may
        // change under it; it doesn't.
        #[allow(clippy::mutable_key_type)]
        let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
        for e in edits {
            let location = self.location(e.span);
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
                let mut value = format!("```t\n{}\n```", note.hover());
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
                    Some(note) => a.program_location(note.def?)?,
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
                let locations: Vec<Location> = spans
                    .into_iter()
                    .filter_map(|s| a.program_location(s))
                    .collect();
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
            let target = renaming(docs, &a).and_then(|(sources, prog, info)| {
                let offset = a
                    .offset(uri, at.position)
                    .ok_or("this file isn't in the program")?;
                let note = crate::rename::target(&sources, &prog, &info, offset)?;
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
            let edits = renaming(docs, &a).and_then(|(sources, prog, info)| {
                let offset = a
                    .offset(uri, at.position)
                    .ok_or("this file isn't in the program")?;
                let note = crate::rename::target(&sources, &prog, &info, offset)?;
                let def = note.def.expect("a target is defined somewhere");
                crate::rename::rename(&sources, &prog, &info, def, &p.new_name)
            });
            Ok(match edits {
                Ok(edits) => Response::new_ok(req.id, a.workspace_edit(&edits)),
                Err(why) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, why),
            })
        }
        DocumentSymbolRequest::METHOD => {
            let p: DocumentSymbolParams = serde_json::from_value(req.params)?;
            let outline = docs.0.get(p.text_document.uri.as_str()).and_then(|src| {
                let prog = crate::lexer::lex(src).and_then(crate::parser::parse).ok()?;
                Some(DocumentSymbolResponse::Nested(outline(src, &prog)))
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
            let Some(src) = docs.0.get(p.text_document.uri.as_str()) else {
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

/// The program `a` is about, parsed and checked for renaming in it.
fn renaming(docs: &Docs, a: &Analysis) -> std::result::Result<(Sources, Program, Info), String> {
    crate::rename::checked(&a.root, &a.text, &|p: &Path| docs.read(p))
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

/// Publishes the diagnostics of every file of the program the open file at
/// `uri` is in: its `version` has just changed. A file that was in the
/// program and no longer is has its diagnostics cleared.
fn publish_all(
    conn: &Connection,
    docs: &Docs,
    (uri, version): (Uri, i32),
    published: &mut HashMap<PathBuf, Vec<Uri>>,
) -> Result<()> {
    let Some(a) = docs.analyse(&uri) else {
        return Ok(());
    };
    let before = published.get(&a.root).cloned().unwrap_or_default();
    for file in &a.uris {
        let diags: Vec<Diagnostic> = diagnose(&a, file).into_iter().map(|(d, _)| d).collect();
        // A file neither open nor with anything to say, now or before,
        // needs nothing published.
        if diags.is_empty() && !docs.0.contains_key(file.as_str()) && !before.contains(file) {
            continue;
        }
        let version = (*file == uri).then_some(version);
        publish(conn, file.clone(), version, diags)?;
    }
    let before = published.insert(a.root.clone(), a.uris.clone());
    for old in before.into_iter().flatten() {
        if !a.uris.contains(&old) {
            publish(conn, old, None, Vec::new())?;
        }
    }
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
    if let Err(e) = &a.checked.result
        && let Some(d) = error_diagnostic(a, uri, e)
    {
        out.push((d, e.fixes.clone()));
    }
    out
}

/// A quick fix for each fix of each diagnostic in the file at `uri` that
/// touches `range`.
fn code_actions(a: &Analysis, uri: &Uri, range: Range) -> Vec<CodeActionOrCommand> {
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
    let methods = prog.methods();
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
    for (i, &(f, parent)) in fns.iter().enumerate().rev() {
        let mut children = std::mem::take(&mut inside[i]);
        sort(&mut children);
        let mut s = fn_symbol(src, f, children);
        let method = i.checked_sub(prog.fns.len()).and_then(|j| methods.get(j));
        match (parent, method) {
            (Some(p), _) => inside[p].push(s),
            (None, Some(&(_, of))) => {
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
            &t.name,
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
        let name = crate::ast::Ident {
            name: header.clone(),
            span: d.self_ty.span,
        };
        top[d.module].push(symbol(
            src,
            &name,
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
        let s = symbol(src, &m.name, detail, SymbolKind::MODULE, m.span, children);
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
    name: &crate::ast::Ident,
    detail: String,
    kind: SymbolKind,
    span: Span,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: name.name.clone(),
        detail: Some(detail),
        kind,
        tags: None,
        deprecated: None,
        range: lsp_range(src, span),
        selection_range: lsp_range(src, name.span),
        children: Some(children),
    }
}

fn fn_symbol(src: &str, f: &FnDecl, children: Vec<DocumentSymbol>) -> DocumentSymbol {
    let params: Vec<&crate::ast::TypeExpr> = f.params.iter().map(|p| &p.ty).collect();
    let detail = format!("fn({}) -> {}", comma_list(&params), f.ret);
    symbol(src, &f.name, detail, SymbolKind::FUNCTION, f.span, children)
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
                    &f.name,
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
                    &v.name,
                    detail,
                    SymbolKind::ENUM_MEMBER,
                    v.span,
                    Vec::new(),
                )
            });
            ("enum", SymbolKind::ENUM, variants.collect())
        }
    };
    let detail = format!("{keyword} {ty}");
    symbol(src, d.name(), detail, kind, d.span(), children)
}

/// The types the checker inferred, and the type arguments it worked out for
/// each use of something generic, inside `range` of the file at `uri`.
fn inlay_hints(a: &Analysis, uri: &Uri, range: Range) -> Vec<InlayHint> {
    let mut notes: Vec<_> = a.info().notes.iter().collect();
    notes.sort_by_key(|n| n.span.start);
    notes
        .iter()
        .filter_map(|n| {
            let hint = n.hint()?;
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
    let related = e
        .notes
        .iter()
        .map(|(span, note)| DiagnosticRelatedInformation {
            location: a.location(*span),
            message: note.clone(),
        })
        .collect::<Vec<_>>();
    Some(Diagnostic {
        range: a.range_in(uri, e.span)?,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("t".to_string()),
        message: format!("{phase}: {}", e.message),
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
    let Some(rest) = uri.as_str().strip_prefix("file://") else {
        return PathBuf::from(uri.as_str());
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
    PathBuf::from(String::from_utf8_lossy(&out).into_owned())
}

/// The `file:` URI of a path, the inverse of `uri_path`.
fn path_uri(path: &Path) -> Uri {
    let s = path.to_string_lossy();
    if !s.starts_with('/')
        && let Ok(uri) = s.parse()
    {
        return uri;
    }
    let mut out = String::from("file://");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out.parse().expect("a file URI")
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
    fn position_counts_utf16_units() {
        let src = "ab\né𝕏x\n";
        let x = src.find('x').unwrap();
        // `é` is 1 unit (2 bytes in UTF-8), `𝕏` is 2 units (4 bytes).
        assert_eq!(position(src, x), Position::new(1, 3));
        assert_eq!(position(src, 0), Position::new(0, 0));
        assert_eq!(position(src, src.len()), Position::new(2, 0));
    }

    #[test]
    fn offset_inverts_position() {
        let src = "ab\né𝕏x\n";
        for (i, _) in src.char_indices().chain([(src.len(), ' ')]) {
            assert_eq!(offset(src, position(src, i)), i);
        }
    }
}
