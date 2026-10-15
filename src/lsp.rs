//! A Language Server Protocol server, so any LSP-capable editor can show T's
//! errors and warnings as you type, and offer their fixes as quick fixes. Run
//! as `t lsp`; it talks JSON-RPC over stdin/stdout.
//!
//! On every open/change it re-checks the whole file (it's tiny and fast) and
//! reports every warning and the first error. Asked for anything else, it
//! checks the file again and answers from what the checker found out
//! (`check::Info`): the fixes of the diagnostics in range as code actions,
//! the type and doc comment of what's under the cursor on hover, where
//! that's defined and where its type is, every place it's named (as
//! references and highlights), and inferred types and type arguments as
//! inlay hints. It renames as `rename.rs` does. The outline of a file comes
//! from parsing it alone, so it's there even when the file doesn't check.
//! It formats a file like `t fmt`, ignoring the editor's formatting
//! options: there's one style. It never runs the program.

use crate::ast::{FnDecl, Program, TypeDecl};
use crate::check::{Info, NoteKind};
use crate::error::{Error, Fix, Lint, Phase, Span, Warning};
use crate::fmt::FmtError;
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
    CodeActionProviderCapability, Diagnostic, DiagnosticSeverity, DiagnosticTag,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams,
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability, InlayHint,
    InlayHintKind, InlayHintLabel, InlayHintParams, Location, MarkupContent, MarkupKind,
    NumberOrString, OneOf, Position, PrepareRenameResponse, PublishDiagnosticsParams, Range,
    ReferenceParams, RenameOptions, RenameParams, ServerCapabilities, SymbolKind,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
    TypeDefinitionProviderCapability, Uri, WorkspaceEdit,
};
use std::collections::HashMap;

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

    // The text of every open file, by URI.
    let mut docs: HashMap<String, String> = HashMap::new();
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
                    let diags = diagnose(&doc.text).into_iter().map(|(d, _)| d).collect();
                    publish(&conn, doc.uri.clone(), Some(doc.version), diags)?;
                    docs.insert(doc.uri.as_str().to_string(), doc.text);
                }
                DidChangeTextDocument::METHOD => {
                    let p: DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
                    // With FULL sync, the last change holds the whole new text.
                    if let Some(change) = p.content_changes.into_iter().last() {
                        let doc = p.text_document;
                        let diags = diagnose(&change.text).into_iter().map(|(d, _)| d).collect();
                        publish(&conn, doc.uri.clone(), Some(doc.version), diags)?;
                        docs.insert(doc.uri.as_str().to_string(), change.text);
                    }
                }
                DidCloseTextDocument::METHOD => {
                    let p: DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
                    docs.remove(p.text_document.uri.as_str());
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

fn respond(req: Request, docs: &HashMap<String, String>) -> Result<Response> {
    match req.method.as_str() {
        CodeActionRequest::METHOD => {
            let p: CodeActionParams = serde_json::from_value(req.params)?;
            let actions = match docs.get(p.text_document.uri.as_str()) {
                Some(src) => code_actions(&p.text_document.uri, src, p.range),
                None => Vec::new(),
            };
            Ok(Response::new_ok(req.id, actions))
        }
        HoverRequest::METHOD => {
            let p: HoverParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let hover = docs.get(at.text_document.uri.as_str()).and_then(|src| {
                let info = crate::check_source(src).info;
                let note = info.note_at(offset(src, at.position))?;
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
                    range: Some(lsp_range(src, note.span)),
                })
            });
            Ok(Response::new_ok(req.id, hover))
        }
        GotoDefinition::METHOD => {
            let p: GotoDefinitionParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let uri = at.text_document.uri;
            let def = docs.get(uri.as_str()).and_then(|src| {
                let info = crate::check_source(src).info;
                let def = info.note_at(offset(src, at.position))?.def?;
                Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range: lsp_range(src, def),
                }))
            });
            Ok(Response::new_ok(req.id, def))
        }
        GotoTypeDefinition::METHOD => {
            let p: GotoDefinitionParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let uri = at.text_document.uri;
            let defs = docs.get(uri.as_str()).and_then(|src| {
                let info = crate::check_source(src).info;
                let spans = type_definitions(&info, offset(src, at.position));
                let locations: Vec<Location> = spans
                    .into_iter()
                    .map(|span| Location::new(uri.clone(), lsp_range(src, span)))
                    .collect();
                (!locations.is_empty()).then_some(GotoDefinitionResponse::Array(locations))
            });
            Ok(Response::new_ok(req.id, defs))
        }
        References::METHOD => {
            let p: ReferenceParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position;
            let uri = at.text_document.uri;
            let refs = docs.get(uri.as_str()).and_then(|src| {
                let info = crate::check_source(src).info;
                let def = info.note_at(offset(src, at.position))?.def?;
                let spans = info.occurrences(def).into_iter();
                let spans = spans.filter(|&s| p.context.include_declaration || s != def);
                let refs = spans.map(|span| Location::new(uri.clone(), lsp_range(src, span)));
                Some(refs.collect::<Vec<_>>())
            });
            Ok(Response::new_ok(req.id, refs))
        }
        DocumentHighlightRequest::METHOD => {
            let p: DocumentHighlightParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position_params;
            let highlights = docs.get(at.text_document.uri.as_str()).and_then(|src| {
                let info = crate::check_source(src).info;
                let def = info.note_at(offset(src, at.position))?.def?;
                let highlights = info
                    .occurrences(def)
                    .into_iter()
                    .map(|span| DocumentHighlight {
                        range: lsp_range(src, span),
                        kind: Some(match span == def {
                            true => DocumentHighlightKind::WRITE,
                            false => DocumentHighlightKind::READ,
                        }),
                    });
                Some(highlights.collect::<Vec<_>>())
            });
            Ok(Response::new_ok(req.id, highlights))
        }
        PrepareRenameRequest::METHOD => {
            let at: TextDocumentPositionParams = serde_json::from_value(req.params)?;
            let Some(src) = docs.get(at.text_document.uri.as_str()) else {
                return Ok(Response::new_ok(req.id, ()));
            };
            let target = crate::rename::checked(src).and_then(|(prog, info)| {
                let note = crate::rename::target(src, &prog, &info, offset(src, at.position))?;
                Ok(note.span)
            });
            Ok(match target {
                Ok(span) => {
                    Response::new_ok(req.id, PrepareRenameResponse::Range(lsp_range(src, span)))
                }
                Err(why) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, why),
            })
        }
        Rename::METHOD => {
            let p: RenameParams = serde_json::from_value(req.params)?;
            let at = p.text_document_position;
            let uri = at.text_document.uri;
            let Some(src) = docs.get(uri.as_str()) else {
                return Ok(Response::new_ok(req.id, ()));
            };
            let edits = crate::rename::checked(src).and_then(|(prog, info)| {
                let note = crate::rename::target(src, &prog, &info, offset(src, at.position))?;
                let def = note.def.expect("a target is defined somewhere");
                crate::rename::rename(src, &prog, &info, def, &p.new_name)
            });
            Ok(match edits {
                Ok(edits) => {
                    let edits = edits
                        .iter()
                        .map(|e| TextEdit::new(lsp_range(src, e.span), e.text.clone()))
                        .collect();
                    Response::new_ok(
                        req.id,
                        WorkspaceEdit {
                            changes: Some(HashMap::from([(uri, edits)])),
                            ..Default::default()
                        },
                    )
                }
                Err(why) => Response::new_err(req.id, ErrorCode::RequestFailed as i32, why),
            })
        }
        DocumentSymbolRequest::METHOD => {
            let p: DocumentSymbolParams = serde_json::from_value(req.params)?;
            let outline = docs.get(p.text_document.uri.as_str()).and_then(|src| {
                let prog = crate::lexer::lex(src).and_then(crate::parser::parse).ok()?;
                Some(DocumentSymbolResponse::Nested(outline(src, &prog)))
            });
            Ok(Response::new_ok(req.id, outline))
        }
        InlayHintRequest::METHOD => {
            let p: InlayHintParams = serde_json::from_value(req.params)?;
            let hints = match docs.get(p.text_document.uri.as_str()) {
                Some(src) => inlay_hints(src, p.range),
                None => Vec::new(),
            };
            Ok(Response::new_ok(req.id, hints))
        }
        Formatting::METHOD => {
            let p: DocumentFormattingParams = serde_json::from_value(req.params)?;
            let Some(src) = docs.get(p.text_document.uri.as_str()) else {
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

/// Every diagnostic for `src`, each with its fixes.
fn diagnose(src: &str) -> Vec<(Diagnostic, Vec<Fix>)> {
    let checked = crate::check_source(src);
    let mut out: Vec<_> = checked
        .warnings
        .iter()
        .map(|w| (warning_diagnostic(w, src), w.fixes.clone()))
        .collect();
    if let Err(e) = &checked.result {
        out.push((error_diagnostic(e, src), e.fixes.clone()));
    }
    out
}

/// A quick fix for each fix of each diagnostic that touches `range`.
fn code_actions(uri: &Uri, src: &str, range: Range) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    for (diag, fixes) in diagnose(src) {
        if diag.range.start > range.end || range.start > diag.range.end {
            continue;
        }
        // The fix to apply without asking, if the editor does that, is the
        // only one there is.
        let preferred = fixes.len() == 1;
        for fix in fixes {
            let edits = fix
                .edits
                .iter()
                .map(|e| TextEdit::new(lsp_range(src, e.span), e.text.clone()))
                .collect();
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: fix.title,
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(uri.clone(), edits)])),
                    ..Default::default()
                }),
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
    if let Type::Adt(symbol, _) = t {
        out.push(symbol);
    }
    for p in t.parts() {
        adts(p, out);
    }
}

/// The outline of `prog`: its functions, structs and enums, with the
/// fields and variants of each type, and the nested functions and local
/// types of each function, under it. In source order.
fn outline(src: &str, prog: &Program) -> Vec<DocumentSymbol> {
    let fns = prog.all_fns();
    let mut inside: Vec<Vec<DocumentSymbol>> = vec![Vec::new(); fns.len()];
    let mut top = Vec::new();
    for (d, f) in prog.all_types() {
        let s = type_symbol(src, d);
        match f {
            Some(f) => inside[f].push(s),
            None => top.push(s),
        }
    }
    // Nested functions come after the ones they're in.
    for (i, &(f, parent)) in fns.iter().enumerate().rev() {
        let mut children = std::mem::take(&mut inside[i]);
        sort(&mut children);
        let s = fn_symbol(src, f, children);
        match parent {
            Some(p) => inside[p].push(s),
            None => top.push(s),
        }
    }
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
/// each use of something generic, inside `range`.
fn inlay_hints(src: &str, range: Range) -> Vec<InlayHint> {
    let mut notes = crate::check_source(src).info.notes;
    notes.sort_by_key(|n| n.span.start);
    notes
        .iter()
        .filter_map(|n| {
            let hint = n.hint()?;
            let pos = position(src, n.span.end);
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

fn error_diagnostic(e: &Error, src: &str) -> Diagnostic {
    let phase = match e.phase {
        Phase::Syntax => "syntax error",
        Phase::Type => "type error",
        Phase::Runtime => "runtime error",
    };
    Diagnostic {
        range: lsp_range(src, e.span),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("t".to_string()),
        message: format!("{phase}: {}", e.message),
        ..Default::default()
    }
}

fn warning_diagnostic(w: &Warning, src: &str) -> Diagnostic {
    Diagnostic {
        range: lsp_range(src, w.span),
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
    }
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
