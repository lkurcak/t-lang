//! A Language Server Protocol server, so any LSP-capable editor can show T's
//! errors as you type. Run as `t lsp`; it talks JSON-RPC over stdin/stdout.
//!
//! For now it only publishes diagnostics: on every open/change it re-checks the
//! whole file (it's tiny and fast) and reports the first error. It never runs
//! the program.

use crate::error::{Error, Phase};
use lsp_server::{Connection, ErrorCode, Message, Notification, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Position, PublishDiagnosticsParams, Range, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn run() -> Result<()> {
    let (conn, io_threads) = Connection::stdio();
    let capabilities = ServerCapabilities {
        // FULL: the editor sends the whole file on every change, not diffs.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    };
    conn.initialize(serde_json::to_value(capabilities)?)?;

    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req)? {
                    break;
                }
                let msg = format!("unsupported request `{}`", req.method);
                let resp = Response::new_err(req.id, ErrorCode::MethodNotFound as i32, msg);
                conn.sender.send(resp.into())?;
            }
            Message::Notification(n) => match n.method.as_str() {
                DidOpenTextDocument::METHOD => {
                    let p: DidOpenTextDocumentParams = serde_json::from_value(n.params)?;
                    let doc = p.text_document;
                    publish(&conn, doc.uri, Some(doc.version), diagnose(&doc.text))?;
                }
                DidChangeTextDocument::METHOD => {
                    let p: DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
                    // With FULL sync, the last change holds the whole new text.
                    if let Some(change) = p.content_changes.last() {
                        let doc = p.text_document;
                        publish(&conn, doc.uri, Some(doc.version), diagnose(&change.text))?;
                    }
                }
                DidCloseTextDocument::METHOD => {
                    let p: DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
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

fn diagnose(src: &str) -> Vec<Diagnostic> {
    match crate::check_source(src) {
        Ok(_) => Vec::new(),
        Err(e) => vec![to_diagnostic(&e, src)],
    }
}

fn to_diagnostic(e: &Error, src: &str) -> Diagnostic {
    let phase = match e.phase {
        Phase::Syntax => "syntax error",
        Phase::Type => "type error",
        Phase::Runtime => "runtime error",
    };
    Diagnostic {
        range: Range::new(position(src, e.span.start), position(src, e.span.end)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("t".to_string()),
        message: format!("{phase}: {}", e.message),
        ..Default::default()
    }
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
}
