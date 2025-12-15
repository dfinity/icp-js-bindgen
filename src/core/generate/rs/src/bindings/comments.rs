/// Escapes doc comment content to prevent comment injection attacks.
/// Replaces `*/` with `*\/` to prevent premature comment termination.
pub(super) fn escape_doc_comment(line: &str) -> String {
    line.replace("*/", r"*\/")
}
