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
/// Entries are distinct per declared name *and* tag list.
///
/// Per name, because a named candid type must get its own declaration even when another type
/// happens to share its tags — TypeScript enums are nominal, so collapsing them leaves the
/// second type referenced everywhere and declared nowhere.
///
/// Per tag list as well, because two candid types can escape to the same identifier: `Map`
/// shadows a global and becomes `Map_`, colliding with a type actually named `Map_`. Matching
/// on the name alone would let the second reuse the first one's enum, leaving its members
/// undeclared while every reference still resolved — invisible to any check, and `undefined`
/// at runtime. Keeping both surfaces the collision instead.
///
/// Anonymous inline variants have no name of their own, so they reuse whichever enum was
/// declared for their tag list first, including one belonging to a named type.
#[derive(Default, Clone)]
pub struct EnumDeclarations {
    declared: Vec<DeclaredEnum>,
}

#[derive(Clone)]
struct DeclaredEnum {
    name: String,
    fields: Vec<Field>,
    decl: TsEnumDecl,
}

impl EnumDeclarations {
    /// The name this variant is *declared* under, if it has been interned already.
    ///
    /// Strict: a named candid type matches only an entry with both its name and its tags. It
    /// must never adopt another type's enum, which is the whole point of interning per name.
    pub fn declared_name(&self, type_name: Option<&str>, fields: &[Field]) -> Option<String> {
        match type_name {
            Some(name) => {
                let declared = candid_type_ident(name).sym.to_string();
                self.declared
                    .iter()
                    .any(|e| e.name == declared && e.fields == fields)
                    .then_some(declared)
            }
            None => self.first_for_tags(fields),
        }
    }

    /// The enum a *reference* to this variant resolves to.
    ///
    /// Lenient where the declaration path is strict, because candid resolves `type B = A`
    /// transitively: the name reaching a conversion can be an alias rather than the type the
    /// enum was declared for, and the alias has no enum of its own.
    ///
    /// Falls back to the name the declaration path would have chosen, so a genuinely missing
    /// enum stays visible to the consistency checks as a dangling reference rather than
    /// aborting the generator.
    pub fn referenced_name(&self, type_name: Option<&str>, fields: &[Field]) -> String {
        self.declared_name(type_name, fields)
            .or_else(|| self.first_for_tags(fields))
            .unwrap_or_else(|| Self::anonymous_name_for(fields))
    }

    /// Whichever enum was declared for this tag list first — what an anonymous variant reuses.
    fn first_for_tags(&self, fields: &[Field]) -> Option<String> {
        self.declared
            .iter()
            .find(|e| e.fields == fields)
            .map(|e| e.name.clone())
    }

    /// The name an enum would be declared under for these tags, whether or not one exists.
    fn anonymous_name_for(fields: &[Field]) -> String {
        let tags: Vec<String> = fields
            .iter()
            .map(|f| match &*f.id {
                Label::Named(name) => name.clone(),
                Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
            })
            .collect();
        candid_type_ident(&format!("Variant_{}", tags.join("_")))
            .sym
            .to_string()
    }

    pub fn insert(&mut self, name: String, fields: &[Field], decl: TsEnumDecl) {
        if !self
            .declared
            .iter()
            .any(|e| e.name == name && e.fields == fields)
        {
            self.declared.push(DeclaredEnum {
                name,
                fields: fields.to_vec(),
                decl,
            });
        }
    }

    /// Every declaration, ordered by name so output is stable.
    pub fn declarations(&self) -> Vec<&TsEnumDecl> {
        let mut entries: Vec<&DeclaredEnum> = self.declared.iter().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries.into_iter().map(|e| &e.decl).collect()
    }
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

/// Suffixes `_` if `name` is a reserved word or a well-known global, so that it can be
/// declared without shadowing or being rejected.
fn escape_reserved(name: String) -> String {
    if KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
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
///
/// The escape is applied *after* capitalization, because that is what turns a harmless
/// basename into a global.
pub fn service_class_name(service_name: &str) -> String {
    escape_reserved(capitalize_first(service_name))
}

/// Names the generated module already occupies — everything it imports, plus the fixed
/// preamble it declares.
///
/// A candid *type* of the same name would collide with one of these. TypeScript merges most
/// of the collisions silently, so the result is not a compile error but a type that claims
/// members the runtime value does not have.
///
/// Deliberately not applied to method names. A method is a property, not a declaration, so it
/// collides with nothing — and renaming one would change the generated client's public API
/// while the wire call kept the candid name.
static MODULE_NAMES: [&str; 22] = [
    // imported from @icp-sdk/core
    "Actor",
    "HttpAgent",
    "HttpAgentOptions",
    "ActorConfig",
    "Agent",
    "ActorSubclass",
    "Principal",
    // imported from the generated declarations
    "idlFactory",
    "_SERVICE",
    // types the preamble declares
    "Option",
    "Some",
    "None",
    "CreateActorOptions",
    // functions the preamble declares. An `enum` or a `class` declares a value as well as a
    // type, so a candid type of one of these names collides with the helper rather than
    // merging with it — two top-level bindings of the same name in one module.
    "some",
    "none",
    "isSome",
    "isNone",
    "unwrap",
    "candid_some",
    "candid_none",
    "record_opt_to_undefined",
    "createActor",
];

/// A candid *type* name as an identifier: escaped against reserved words, the globals it
/// would shadow, and the names the generated module already occupies.
pub fn candid_type_ident(name: &str) -> Ident {
    if MODULE_NAMES.contains(&name) {
        get_ident(&format!("{name}_"))
    } else {
        get_ident_guarded(name)
    }
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

    #[test]
    fn service_class_name_capitalizes() {
        assert_eq!(service_class_name("backend"), "Backend");
        assert_eq!(service_class_name("hello_world"), "Hello_world");
    }

    /// A multi-byte leading character used to be sliced at byte 1, panicking the generator.
    #[test]
    fn service_class_name_handles_multibyte_leading_character() {
        assert_eq!(service_class_name("ünicode"), "Ünicode");
        assert_eq!(service_class_name("日本語"), "日本語");
        assert_eq!(service_class_name(""), "");
    }

    /// The escape has to be applied *after* capitalization, because that is what turns a
    /// harmless basename into a global.
    #[test]
    fn service_class_name_escapes_globals() {
        assert_eq!(service_class_name("map"), "Map_");
        assert_eq!(service_class_name("set"), "Set_");
        assert_eq!(service_class_name("error"), "Error_");
        // Not a reserved word once capitalized, so left alone.
        assert_eq!(service_class_name("class"), "Class");
    }

    /// A candid type may be named after something the module imports or the preamble declares.
    #[test]
    fn candid_type_ident_escapes_names_the_module_occupies() {
        assert_eq!(&*candid_type_ident("Option").sym, "Option_");
        assert_eq!(&*candid_type_ident("Agent").sym, "Agent_");
        assert_eq!(&*candid_type_ident("Principal").sym, "Principal_");
        assert_eq!(&*candid_type_ident("Status").sym, "Status");
    }
}
