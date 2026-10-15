//! A Language Server Protocol server, so any LSP-capable editor can show T's
//! errors and warnings as you type, and offer their fixes as quick fixes. Run
//! as `t lsp`; it talks JSON-RPC over stdin/stdout.
//!
//! On every open/change it re-checks the whole file (it's tiny and fast) and
//! reports every warning and the first error. Asked for anything else, it
//! checks the file again and answers from what the checker found out
//! (`check::Info`): the fixes of the diagnostics in range as code actions,
//! the type of what's under the cursor on hover, where that's defined, and
//! inferred types and type arguments as inlay hints. It formats a file
//! like `t fmt`, ignoring the editor's formatting options: there's one
//! style. It never runs the program.

use crate::check::Note;
use crate::error::{Error, Fix, Lint, Phase, Span, Warning};
use crate::fmt::FmtError;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, Formatting, GotoDefinition, HoverRequest, InlayHintRequest, Request as _,
};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, Diagnostic, DiagnosticSeverity, DiagnosticTag,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, InlayHint, InlayHintKind, InlayHintLabel,
    InlayHintParams, Location, MarkupContent, MarkupKind, NumberOrString, OneOf, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, Uri, WorkspaceEdit,
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
                let notes = crate::check_source(src).info.notes;
                let note = note_at(&notes, src, at.position)?;
                Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("```t\n{}\n```", note.hover()),
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
                let def = note_at(&crate::check_source(src).info.notes, src, at.position)?.def?;
                Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range: lsp_range(src, def),
                }))
            });
            Ok(Response::new_ok(req.id, def))
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

/// What's at `pos`: the innermost name there, if there's one.
fn note_at<'n>(notes: &'n [Note], src: &str, pos: Position) -> Option<&'n Note> {
    let at = offset(src, pos);
    notes
        .iter()
        .filter(|n| n.span.contains(at))
        .min_by_key(|n| n.span.end - n.span.start)
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
