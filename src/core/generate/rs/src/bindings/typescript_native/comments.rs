use super::conversion_functions_generator::TopLevelNodes;
use crate::bindings::comments::escape_doc_comment;
use swc_core::common::{
    BytePos, DUMMY_SP, Span,
    comments::{Comment, CommentKind, Comments},
};

// Simple monotonic position source for synthetic spans
pub struct PosCursor {
    cur: BytePos,
}
impl PosCursor {
    pub fn new() -> Self {
        Self { cur: BytePos(1) }
    }
    pub fn new_synthetic_span(&mut self) -> Span {
        let lo = self.cur;
        self.cur = BytePos(self.cur.0 + 1);
        Span::new(lo, lo)
    }
}

fn make_comment(docs: &[String]) -> Option<Comment> {
    if docs.is_empty() {
        None
    } else {
        // Join all doc lines into a single block comment, with each line prefixed by a space
        let mut comment_text = String::from("*\n");
        for line in docs {
            comment_text.push_str(&format!(" * {}\n", escape_doc_comment(line)));
        }

        // Add a space at the end to align the block comment final line ("*/") properly
        comment_text.push(' ');

        Some(Comment {
            span: DUMMY_SP,
            kind: CommentKind::Block,
            text: comment_text.into(),
        })
    }
}

pub fn add_comments(top_level_nodes: &mut TopLevelNodes, docs: &[String]) -> Span {
    let (_, comments, cursor) = top_level_nodes;
    match docs.len() {
        0 => DUMMY_SP,
        _ => {
            let d = make_comment(docs);
            let span = cursor.new_synthetic_span();
            if let Some(d) = d {
                comments.add_leading(span.lo, d);
            }
            span
        }
    }
}
