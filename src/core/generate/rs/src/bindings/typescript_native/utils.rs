use candid::types::Field;
use std::collections::HashMap;
use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::source_map::SourceMap;
use swc_core::common::sync::Lrc;
use swc_core::ecma::ast::TsEnumDecl;
use swc_core::ecma::{
    ast::Module,
    codegen::{Config, Emitter, text_writer::JsWriter, text_writer::WriteJs},
};

/// Unit-variant enums interned by Candid field list. Each named type with the
/// same tags gets its own declaration; anonymous variants reuse the first match.
pub type EnumDeclarations = HashMap<Vec<Field>, Vec<(TsEnumDecl, String)>>;

pub fn render_ast(module: &Module, comments: &SingleThreadedComments) -> String {
    let mut buf = vec![];
    let cm = Lrc::new(SourceMap::default());
    {
        let writer = JsWriter::new(cm.clone(), "\n", &mut buf, None);
        let writer = NewlineAfterBlockComments::new(writer);
        let mut emitter = Emitter {
            cfg: Config::default().with_minify(false),
            cm: cm.clone(),
            comments: Some(&comments),
            wr: Box::new(writer),
        };

        emitter.emit_module(module).unwrap();
    }

    String::from_utf8(buf).unwrap()
}

// Writer wrapper to enforce a newline after block comments so following tokens don't begin on the same line
struct NewlineAfterBlockComments<W: WriteJs> {
    inner: W,
    suppress_space_after_comment: bool,
}

impl<W: WriteJs> NewlineAfterBlockComments<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            suppress_space_after_comment: false,
        }
    }
}

impl<W: WriteJs> WriteJs for NewlineAfterBlockComments<W> {
    #[inline]
    fn increase_indent(&mut self) -> swc_core::ecma::codegen::Result {
        self.inner.increase_indent()
    }
    #[inline]
    fn decrease_indent(&mut self) -> swc_core::ecma::codegen::Result {
        self.inner.decrease_indent()
    }
    #[inline]
    fn write_semi(
        &mut self,
        span: Option<swc_core::common::Span>,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_semi(span)
    }
    #[inline]
    fn write_space(&mut self) -> swc_core::ecma::codegen::Result {
        if self.suppress_space_after_comment {
            self.suppress_space_after_comment = false;
            return Ok(());
        }
        self.inner.write_space()
    }
    #[inline]
    fn write_keyword(
        &mut self,
        span: Option<swc_core::common::Span>,
        s: &'static str,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_keyword(span, s)
    }
    #[inline]
    fn write_operator(
        &mut self,
        span: Option<swc_core::common::Span>,
        s: &str,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_operator(span, s)
    }
    #[inline]
    fn write_param(&mut self, s: &str) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_param(s)
    }
    #[inline]
    fn write_property(&mut self, s: &str) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_property(s)
    }
    #[inline]
    fn write_line(&mut self) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_line()
    }
    #[inline]
    fn write_lit(
        &mut self,
        span: swc_core::common::Span,
        s: &str,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_lit(span, s)
    }
    #[inline]
    fn write_comment(&mut self, s: &str) -> swc_core::ecma::codegen::Result {
        if s.contains('\n') {
            let mut iter = s.split('\n').peekable();
            if let Some(first) = iter.next() {
                self.inner.write_comment(first)?;
            }
            for rest in iter {
                self.inner.write_line()?;
                self.inner.write_comment(rest)?;
            }
        } else {
            self.inner.write_comment(s)?;
        }
        // Ensure next token starts on a new, properly indented line after a block comment
        if s.trim_end().ends_with("*/") {
            self.suppress_space_after_comment = true; // skip a single space that codegen may emit next
            self.inner.write_line()?;
        }
        Ok(())
    }
    #[inline]
    fn write_str_lit(
        &mut self,
        span: swc_core::common::Span,
        s: &str,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_str_lit(span, s)
    }
    #[inline]
    fn write_str(&mut self, s: &str) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_str(s)
    }
    #[inline]
    fn write_symbol(
        &mut self,
        span: swc_core::common::Span,
        s: &str,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_symbol(span, s)
    }
    #[inline]
    fn write_punct(
        &mut self,
        span: Option<swc_core::common::Span>,
        s: &'static str,
    ) -> swc_core::ecma::codegen::Result {
        self.suppress_space_after_comment = false;
        self.inner.write_punct(span, s)
    }
    #[inline]
    fn care_about_srcmap(&self) -> bool {
        self.inner.care_about_srcmap()
    }
    #[inline]
    fn add_srcmap(&mut self, pos: swc_core::common::BytePos) -> swc_core::ecma::codegen::Result {
        self.inner.add_srcmap(pos)
    }
    #[inline]
    fn commit_pending_semi(&mut self) -> swc_core::ecma::codegen::Result {
        self.inner.commit_pending_semi()
    }
    #[inline(always)]
    fn can_ignore_invalid_unicodes(&mut self) -> bool {
        self.inner.can_ignore_invalid_unicodes()
    }
}

use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::*;

pub static KEYWORDS: [&str; 125] = [
    // Original JavaScript keywords
    "abstract",
    "arguments",
    "await",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "double",
    "else",
    "enum",
    "eval",
    "export",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "function",
    "goto",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "int",
    "interface",
    "let",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "volatile",
    "while",
    "with",
    "yield",
    // TypeScript primitive types
    "any",
    "unknown",
    "never",
    "undefined",
    "object",
    "symbol",
    "bigint",
    "number",
    "string",
    // Utility types
    "Partial",
    "Required",
    "Readonly",
    "Record",
    "Pick",
    "Omit",
    "Exclude",
    "Extract",
    "NonNullable",
    "Parameters",
    "ConstructorParameters",
    "ReturnType",
    "InstanceType",
    "ThisParameterType",
    "OmitThisParameter",
    "ThisType",
    "Uppercase",
    "Lowercase",
    "Capitalize",
    "Uncapitalize",
    // Common built-in types/interfaces
    "Array",
    "ReadonlyArray",
    "Function",
    "Date",
    "Error",
    "Promise",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "Iterator",
    "IterableIterator",
    "Generator",
    "RegExp",
    "ArrayBuffer",
    "DataView",
    "Float32Array",
    "Float64Array",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Uint8Array",
    "Uint16Array",
    "Uint32Array",
    "Uint8ClampedArray",
    "BigInt64Array",
    "BigUint64Array",
    // Common global objects
    "Math",
    "JSON",
    "console",
    "document",
    "window",
];

pub fn get_ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
}

pub fn get_typescript_ident(name: &str, filter_keywords: bool) -> String {
    // Handle empty names by returning a quoted empty string
    if name.is_empty() {
        return "\"\"".to_string();
    }

    if filter_keywords && KEYWORDS.contains(&name) {
        return format!("{}_", name);
    }

    if name.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
        // If the name contains non-alphanumeric characters (except underscore),
        // or contains quotes, we need to quote it to make it a valid TypeScript property name
        format!("'{}'", name.escape_debug())
    } else {
        name.to_string()
    }
}

pub fn contains_unicode_characters(name: &str) -> bool {
    name != get_typescript_ident(name, false)
}

pub fn get_ident_guarded(name: &str) -> Ident {
    let ident_name = get_typescript_ident(name, true);
    get_ident(&ident_name)
}

pub fn get_ident_guarded_keyword_ok(name: &str) -> Ident {
    let ident_name: String = get_typescript_ident(name, false);
    get_ident(&ident_name)
}

/// Intern a unit-variant enum. Named types always get a declaration under their
/// own (escaped) name, even when another type already used the same tags.
/// Anonymous variants reuse the first enum interned for that field list.
pub fn intern_unit_variant_enum(
    enum_declarations: &mut EnumDeclarations,
    fields: &[Field],
    type_name: Option<&str>,
    field_info: Vec<(String, swc_core::common::Span)>,
) -> String {
    let requested_name = match type_name {
        Some(name) => name.to_string(),
        None => {
            let field_names: Vec<String> =
                field_info.iter().map(|(name, _)| name.clone()).collect();
            format!("Variant_{}", field_names.join("_"))
        }
    };
    let enum_ident = get_ident_guarded(&requested_name);
    let declared_name = enum_ident.sym.to_string();

    let enums = enum_declarations.entry(fields.to_vec()).or_default();

    if type_name.is_none()
        && let Some((_, existing_name)) = enums.first()
    {
        return existing_name.clone();
    }

    if let Some((_, existing_name)) = enums.iter().find(|(_, name)| name == &declared_name) {
        return existing_name.clone();
    }

    // Reserved words are valid enum member names and valid in member-access
    // position (`Status.new`), so they must NOT be escaped here.
    let members = field_info
        .into_iter()
        .map(|(member_name, span)| TsEnumMember {
            span,
            id: TsEnumMemberId::Ident(get_ident_guarded_keyword_ok(&member_name)),
            init: Some(Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: member_name.into(),
                raw: None,
            })))),
        })
        .collect();
    let enum_decl = TsEnumDecl {
        span: DUMMY_SP,
        declare: false,
        is_const: false,
        id: enum_ident,
        members,
    };
    enums.push((enum_decl, declared_name.clone()));
    declared_name
}

pub fn unit_variant_enum_name(enum_declarations: &EnumDeclarations, fields: &[Field]) -> String {
    enum_declarations
        .get(&fields.to_vec())
        .and_then(|enums| enums.first())
        .map(|(_, name)| name.clone())
        .expect("unit variant enum should have been interned before conversion")
}

pub fn sorted_enum_decls(enum_declarations: &EnumDeclarations) -> Vec<TsEnumDecl> {
    let mut items: Vec<&(TsEnumDecl, String)> = enum_declarations.values().flatten().collect();
    items.sort_by(|a, b| a.1.cmp(&b.1));
    items.into_iter().map(|(decl, _)| decl.clone()).collect()
}
