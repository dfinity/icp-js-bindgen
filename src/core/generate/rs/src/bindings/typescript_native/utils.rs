use candid::types::{Field, Label};
use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::source_map::SourceMap;
use swc_core::common::sync::Lrc;
use swc_core::ecma::ast::TsEnumDecl;
use swc_core::ecma::{
    ast::Module,
    codegen::{Config, Emitter, text_writer::JsWriter, text_writer::WriteJs},
};

/// The `enum` declarations lowered from all-null candid variants.
///
/// A *named* candid type gets an entry of its own, keyed by that name: TypeScript enums are
/// nominal, so two named types with identical tags cannot share one declaration without the
/// second going undeclared. Anonymous inline variants have no name to be declared under and
/// reuse whichever enum was interned for their tag list.
#[derive(Default, Clone)]
pub struct EnumDeclarations {
    declared: Vec<DeclaredEnum>,
}

#[derive(Clone)]
struct DeclaredEnum {
    /// The candid type the enum was declared for; absent for an anonymous variant.
    candid_name: Option<String>,
    /// The identifier it is declared under.
    name: String,
    fields: Vec<Field>,
    decl: TsEnumDecl,
}

impl EnumDeclarations {
    /// The name this variant is declared under, if it has been interned.
    ///
    /// A named type matches on its *candid* name, so it never adopts another named type's
    /// enum; an anonymous one falls back to the tag list, as before.
    pub fn declared_name(&self, type_name: Option<&str>, fields: &[Field]) -> Option<String> {
        match type_name {
            Some(name) => self
                .declared
                .iter()
                .find(|e| e.candid_name.as_deref() == Some(name))
                .map(|e| e.name.clone()),
            None => self.first_for_tags(fields),
        }
    }

    /// The enum a reference to this variant resolves to.
    ///
    /// Falls back to the tag list, since a type the caller could not name — an inline variant
    /// — has no entry of its own; then to the name the declaration path would choose, so a
    /// missing enum surfaces as a dangling reference rather than a panic.
    pub fn referenced_name(&self, type_name: Option<&str>, fields: &[Field]) -> String {
        self.declared_name(type_name, fields)
            .or_else(|| self.first_for_tags(fields))
            .unwrap_or_else(|| anonymous_enum_name(fields))
    }

    /// Whichever enum was declared for this tag list first — what an anonymous variant reuses.
    fn first_for_tags(&self, fields: &[Field]) -> Option<String> {
        self.declared
            .iter()
            .find(|e| e.fields == fields)
            .map(|e| e.name.clone())
    }

    pub fn insert(
        &mut self,
        candid_name: Option<&str>,
        name: String,
        fields: &[Field],
        decl: TsEnumDecl,
    ) {
        let already_declared = match candid_name {
            Some(candid_name) => self
                .declared
                .iter()
                .any(|e| e.candid_name.as_deref() == Some(candid_name)),
            None => self
                .declared
                .iter()
                .any(|e| e.name == name && e.fields == fields),
        };
        if already_declared {
            return;
        }
        // An inline variant with these tags may already be declared under exactly this
        // identifier. That is the same enum, so the named type takes it over rather than
        // declaring a second one, and the module reads the same whichever came first.
        if let Some(candid_name) = candid_name
            && let Some(entry) = self
                .declared
                .iter_mut()
                .find(|e| e.candid_name.is_none() && e.name == name && e.fields == fields)
        {
            entry.candid_name = Some(candid_name.to_string());
            entry.decl = decl;
            return;
        }
        self.declared.push(DeclaredEnum {
            candid_name: candid_name.map(str::to_string),
            name,
            fields: fields.to_vec(),
            decl,
        });
    }

    /// Every declaration, ordered by name so output is stable.
    pub fn declarations(&self) -> Vec<&TsEnumDecl> {
        let mut entries: Vec<&DeclaredEnum> = self.declared.iter().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries.into_iter().map(|e| &e.decl).collect()
    }
}

/// The identifier an anonymous inline variant is declared under, derived from its tags.
pub fn anonymous_enum_name(fields: &[Field]) -> String {
    let tags: Vec<String> = fields
        .iter()
        .map(|f| match &*f.id {
            Label::Named(name) => name.clone(),
            Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
        })
        .collect();
    get_ident_guarded(&format!("Variant_{}", tags.join("_")))
        .sym
        .to_string()
}

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
