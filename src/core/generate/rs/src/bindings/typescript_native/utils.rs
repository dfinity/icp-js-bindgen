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

pub type EnumDeclarations = HashMap<Vec<Field>, (TsEnumDecl, String)>;

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

/// Fallback for a name that sanitizes down to nothing at all.
const EMPTY_IDENT_FALLBACK: &str = "_";

/// Whether `c` may start a TypeScript identifier.
///
/// ECMAScript `IdentifierStart` is `ID_Start` plus `$` and `_`. The `XID_*` variants are
/// used here because they are closed under normalization; they are marginally stricter
/// than `ID_*`, which only ever means a name is sanitized that could have been left
/// alone — never that an invalid name is accepted.
fn is_ident_start(c: char) -> bool {
    c == '_' || c == '$' || unicode_ident::is_xid_start(c)
}

/// Whether `c` may appear in a TypeScript identifier after the first character.
fn is_ident_continue(c: char) -> bool {
    c == '$' || unicode_ident::is_xid_continue(c)
}

/// Whether `name` can be used verbatim as an identifier in a binding position.
fn is_valid_binding_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => is_ident_start(first) && chars.all(is_ident_continue),
        None => false,
    }
}

/// Converts `name` into an identifier that is legal in a *binding* position — a
/// declaration name (`class X`, `interface X`, `enum X`) or a type reference.
///
/// This is the counterpart to [`get_typescript_ident`], which *quotes* names it cannot
/// use verbatim. Quoting is correct in *property* position, where `'my-field': bigint`
/// is legal TypeScript, and a syntax error in binding position, where `'my-field'` is a
/// string literal rather than an identifier. The two must therefore not share a path.
///
/// **This function is the identity on names that are already legal identifiers.** That
/// property is load-bearing: the generated interface name is derived from the raw `.did`
/// basename and is a documented part of the public API (`<service-name>Interface`), and
/// Candid restricts type ids to `(letter | '_') (letter | digit | '_')*`, so every name
/// reaching here from a well-formed `.did` file is passed through untouched.
///
/// Illegal characters are collapsed to a single `_` per run, so `my-backend` and
/// `my.backend` both yield `my_backend` — the same output as if the file had been named
/// `my_backend.did`. A leading character that is legal only in continuation position
/// (a digit, say) is prefixed with `_`.
pub fn binding_ident_name(name: &str) -> String {
    if is_valid_binding_ident(name) {
        return name.to_string();
    }

    let mut sanitized = String::with_capacity(name.len());
    let mut in_separator_run = false;
    for c in name.chars() {
        if is_ident_continue(c) {
            sanitized.push(c);
            in_separator_run = false;
        } else if !in_separator_run {
            sanitized.push('_');
            in_separator_run = true;
        }
    }

    // Every retained character is valid in continuation position, but the first one
    // additionally has to be valid in *start* position.
    match sanitized.chars().next() {
        Some(first) if !is_ident_start(first) => format!("_{sanitized}"),
        Some(_) => sanitized,
        None => EMPTY_IDENT_FALLBACK.to_string(),
    }
}

/// Suffixes `_` if `name` is a reserved word or a well-known global, so that it can be
/// declared without shadowing or being rejected.
fn escape_reserved(name: String) -> String {
    if KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

/// Identifier for something *declared* in binding position: sanitized, then escaped if
/// it collides with a reserved word.
pub fn binding_ident(name: &str) -> Ident {
    get_ident(&escape_reserved(binding_ident_name(name)))
}

/// Uppercases the first character, respecting `char` boundaries.
fn capitalize_first(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The identifier of the generated actor class, e.g. `hello_world` -> `Hello_world`.
///
/// Single source of truth: the class declaration, the `createActor` return type and the
/// `new …()` call all resolve it through here, so they cannot disagree. They previously
/// derived it independently, and the reserved-word escape was applied to the declaration
/// only — so `map.did` declared `class Map_` but constructed the global `Map`.
pub fn service_class_name(service_name: &str) -> String {
    escape_reserved(capitalize_first(&binding_ident_name(service_name)))
}

/// Renders `name` for use in *property* position — an object or interface member key,
/// or an enum member id — quoting it when it is not a bare identifier.
///
/// Use [`binding_ident_name`] instead for declaration names and type references, where a
/// quoted string is a syntax error rather than an escape.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The identity property that keeps existing generated output unchanged.
    #[test]
    fn binding_ident_name_is_identity_on_legal_identifiers() {
        for name in [
            "backend",
            "hello_world",
            "my_backend",
            "reserved_words",
            "_leading_underscore",
            "trailing_",
            "with2digits",
            "$dollar",
            "ünicode",
            "Variant_a_b",
        ] {
            assert_eq!(
                binding_ident_name(name),
                name,
                "should be untouched: {name}"
            );
        }
    }

    #[test]
    fn binding_ident_name_sanitizes_illegal_characters() {
        // A run of illegal characters collapses to a single `_`, so a dashed name yields
        // exactly what the underscored filename would have.
        assert_eq!(binding_ident_name("my-backend"), "my_backend");
        assert_eq!(binding_ident_name("my.backend"), "my_backend");
        assert_eq!(binding_ident_name("my backend"), "my_backend");
        assert_eq!(binding_ident_name("my--backend"), "my_backend");
        assert_eq!(binding_ident_name("my.-. backend"), "my_backend");
        assert_eq!(binding_ident_name("üni-code"), "üni_code");
        assert_eq!(binding_ident_name("my-backend-"), "my_backend_");
    }

    #[test]
    fn binding_ident_name_fixes_illegal_leading_character() {
        assert_eq!(binding_ident_name("2fa"), "_2fa");
        assert_eq!(binding_ident_name("-leading"), "_leading");
        assert_eq!(binding_ident_name("123"), "_123");
    }

    #[test]
    fn binding_ident_name_always_returns_a_legal_identifier() {
        for name in [
            "",
            "-",
            "---",
            "...",
            "2fa",
            "my-backend",
            "my.backend",
            "ünicode",
            "üni-code",
            "🎉",
            "🎉-party",
            "a\"b",
            "a'b",
            "a\\b",
            "\n",
        ] {
            let sanitized = binding_ident_name(name);
            assert!(
                is_valid_binding_ident(&sanitized),
                "{name:?} sanitized to {sanitized:?}, which is not a legal identifier"
            );
        }
    }

    #[test]
    fn service_class_name_capitalizes() {
        assert_eq!(service_class_name("backend"), "Backend");
        assert_eq!(service_class_name("hello_world"), "Hello_world");
        assert_eq!(service_class_name("my-backend"), "My_backend");
    }

    /// A multi-byte leading character used to be sliced at byte 1, panicking.
    #[test]
    fn service_class_name_handles_multibyte_leading_character() {
        assert_eq!(service_class_name("ünicode"), "Ünicode");
        assert_eq!(service_class_name("日本語"), "日本語");
        assert_eq!(service_class_name("🎉"), "_");
    }

    /// The reserved-word escape has to be applied *after* capitalization, because that is
    /// what can turn a harmless basename into a global (`map` -> `Map`).
    #[test]
    fn service_class_name_escapes_reserved_words() {
        assert_eq!(service_class_name("map"), "Map_");
        assert_eq!(service_class_name("set"), "Set_");
        assert_eq!(service_class_name("record"), "Record_");
        assert_eq!(service_class_name("error"), "Error_");
        assert_eq!(service_class_name("promise"), "Promise_");
        // Not a reserved word once capitalized, so left alone.
        assert_eq!(service_class_name("class"), "Class");
    }

    #[test]
    fn binding_ident_escapes_reserved_words() {
        assert_eq!(&*binding_ident("Map").sym, "Map_");
        assert_eq!(
            &*binding_ident("Variant_other_my-tag").sym,
            "Variant_other_my_tag"
        );
        assert_eq!(&*binding_ident("Variant_a_b").sym, "Variant_a_b");
    }

    /// Property position keeps quoting; binding position must not.
    #[test]
    fn property_and_binding_paths_differ() {
        assert_eq!(get_typescript_ident("my-field", false), "'my-field'");
        assert_eq!(binding_ident_name("my-field"), "my_field");
    }

    #[test]
    fn declarations_specifier_matches_the_written_basename() {
        // The JavaScript layer writes `<basename>.did.js`/`.did.d.ts`, so the specifier must
        // carry the basename through verbatim.
        for name in ["backend", "my-backend", "my.backend", "ünicode"] {
            assert_eq!(
                super::super::preamble::imports::declarations_module_specifier(name),
                format!("./declarations/{name}.did")
            );
        }
    }
}
