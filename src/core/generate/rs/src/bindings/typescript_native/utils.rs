use candid::types::{Field, Label, Type, TypeEnv, TypeInner};
use std::collections::HashSet;
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
/// Entries are distinct per declared name *and* tag list: per name because enums are nominal,
/// per tag list because two candid types can escape to the same identifier (`Map` and `Map_`
/// both become `Map_`), where a shared entry would give one the other's members.
///
/// Anonymous variants reuse whichever enum was declared for their tag list first.
#[derive(Default, Clone)]
pub struct EnumDeclarations {
    declared: Vec<DeclaredEnum>,
    /// The identifier every *named* candid type declares under, with its tag list when the
    /// type is itself an all-null variant. An anonymous variant's derived name steps aside
    /// from these — except where the tags are the same, since the two are then one enum.
    reserved: Vec<(String, Option<Vec<Field>>)>,
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
    pub fn new(env: &TypeEnv) -> Self {
        Self {
            declared: Vec::new(),
            reserved: env
                .0
                .iter()
                .map(|(id, ty)| (candid_type_ident(id).sym.to_string(), enum_tags(ty)))
                .collect(),
        }
    }

    /// The name this variant is declared under, if it has been interned.
    ///
    /// A named type matches on its *candid* name, so it never adopts another type's enum —
    /// not even one whose identifier it shares. Two candid types that escape to the same
    /// identifier each declare their own, which is a duplicate declaration the module checks
    /// reject rather than a merge nothing can see.
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
    /// Falls back to the tag list, since candid resolves `type B = A` transitively and an
    /// alias has no enum of its own; then to the name the declaration path would choose, so a
    /// missing enum surfaces as a dangling reference rather than a panic.
    pub fn referenced_name(&self, type_name: Option<&str>, fields: &[Field]) -> String {
        self.declared_name(type_name, fields)
            .or_else(|| self.first_for_tags(fields))
            .unwrap_or_else(|| self.anonymous_name_for(fields))
    }

    /// Whichever enum was declared for this tag list first — what an anonymous variant reuses.
    fn first_for_tags(&self, fields: &[Field]) -> Option<String> {
        self.declared
            .iter()
            .find(|e| e.fields == fields)
            .map(|e| e.name.clone())
    }

    /// The name an enum would be declared under for these tags, whether or not one exists.
    ///
    /// Escaped like any type name, then suffixed while a *named* candid type of different
    /// shape holds the identifier: `type Variant_x = variant { y }` beside an inline
    /// `variant { x }` would otherwise declare `Variant_x` twice with different members. A
    /// named `variant { x }` of that name is the same enum, so the inline variant shares it
    /// instead. Depends on the type environment alone, so the outcome is the same whichever
    /// of the two is visited first.
    pub fn anonymous_name_for(&self, fields: &[Field]) -> String {
        let tags: Vec<String> = fields
            .iter()
            .map(|f| match &*f.id {
                Label::Named(name) => name.clone(),
                Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
            })
            .collect();
        let mut name = candid_type_ident(&format!("Variant_{}", tags.join("_")))
            .sym
            .to_string();
        while self
            .reserved
            .iter()
            .any(|(taken, tags)| *taken == name && tags.as_deref() != Some(fields))
        {
            name.push('_');
        }
        name
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
        if let Some(candid_name) = candid_name {
            let adopted = self
                .declared
                .iter_mut()
                .find(|e| e.candid_name.is_none() && e.name == name && e.fields == fields);
            if let Some(entry) = adopted {
                entry.candid_name = Some(candid_name.to_string());
                entry.decl = decl;
                return;
            }
        }
        {
            self.declared.push(DeclaredEnum {
                candid_name: candid_name.map(str::to_string),
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

/// The tags of a candid type that lowers to an `enum`: a variant whose every alternative is
/// `null`. Anything else, including an alias to such a variant, has none.
fn enum_tags(ty: &Type) -> Option<Vec<Field>> {
    match ty.as_ref() {
        TypeInner::Variant(fs)
            if !fs.is_empty() && fs.iter().all(|f| matches!(f.ty.as_ref(), TypeInner::Null)) =>
        {
            Some(fs.clone())
        }
        _ => None,
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
/// The class declaration, the `createActor` return type and the `new …()` call all resolve
/// the name through here, so they cannot disagree.
///
/// The escape runs after capitalization, since that is what turns a basename into a global.
/// A candid type can also be named exactly this — `governance.did` holding
/// `type Governance` — and TypeScript would merge the class with the interface declared for
/// it, so the class steps aside with `_` while a candid type holds the name, the way it does
/// for a basename that capitalizes onto a global.
pub fn service_class_name(service_name: &str, env: &TypeEnv) -> String {
    let capitalized = capitalize_first(service_name);
    let mut name = if MODULE_NAMES.contains(&capitalized.as_str()) {
        format!("{capitalized}_")
    } else {
        escape_reserved(capitalized)
    };
    let taken: HashSet<String> = env
        .0
        .keys()
        .map(|id| candid_type_ident(id).sym.to_string())
        .collect();
    while taken.contains(&name) {
        name.push('_');
    }
    name
}

/// Names the generated module already occupies — everything it imports, plus the fixed
/// preamble it declares.
///
/// A candid *type* of the same name collides with one of these, and TypeScript merges most
/// such collisions silently into a type claiming members the value lacks.
///
/// Not applied to method names: a method is a property, not a declaration, so it collides
/// with nothing.
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

/// The local alias under which the module imports the candid shape of a type.
///
/// The `_` prefix separates that shape from the native type declared alongside it. It is not
/// enough on its own: the module already imports the service type as `_SERVICE`, so a candid
/// type named `SERVICE` would bind that local twice and the actor would be typed by the
/// user's type instead of the service.
pub fn candid_import_local(id: &str) -> String {
    let local = format!("_{id}");
    if MODULE_NAMES.contains(&local.as_str()) {
        format!("{local}_")
    } else {
        local
    }
}

/// Names TypeScript accepts as identifiers but not as a type, although they are fine
/// everywhere else a candid name lands: `as` cannot name a type alias, and `keyof`,
/// `readonly`, `infer` and `unique` declare fine but parse as the type operator wherever the
/// type is referenced. `as(): Promise<void>` is a legal method.
static TYPE_NAME_RESERVED: [&str; 5] = ["as", "infer", "keyof", "readonly", "unique"];

/// A candid *type* name as an identifier: escaped against reserved words, the globals it
/// would shadow, and the names the generated module already occupies.
pub fn candid_type_ident(name: &str) -> Ident {
    if MODULE_NAMES.contains(&name) || TYPE_NAME_RESERVED.contains(&name) {
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
    fn candid_import_local_prefixes_the_candid_shape() {
        assert_eq!(candid_import_local("Outcome"), "_Outcome");
    }

    /// `_SERVICE` is the local the module already binds for the service type.
    #[test]
    fn candid_import_local_steps_aside_for_the_service_type() {
        assert_eq!(candid_import_local("SERVICE"), "_SERVICE_");
    }

    /// A candid type can hold the name the class would take; TypeScript would merge the two.
    #[test]
    fn service_class_name_steps_aside_for_a_candid_type_of_that_name() {
        let mut env = TypeEnv::new();
        env.0
            .insert("Governance".to_string(), TypeInner::Nat.into());
        assert_eq!(service_class_name("governance", &env), "Governance_");
        env.0
            .insert("Governance_".to_string(), TypeInner::Nat.into());
        assert_eq!(service_class_name("governance", &env), "Governance__");
    }

    #[test]
    fn service_class_name_capitalizes() {
        assert_eq!(service_class_name("backend", &TypeEnv::new()), "Backend");
        assert_eq!(
            service_class_name("hello_world", &TypeEnv::new()),
            "Hello_world"
        );
    }

    /// Capitalization is by `char`: slicing at byte 1 splits a multi-byte boundary.
    #[test]
    fn service_class_name_handles_multibyte_leading_character() {
        assert_eq!(service_class_name("ünicode", &TypeEnv::new()), "Ünicode");
        assert_eq!(service_class_name("日本語", &TypeEnv::new()), "日本語");
        assert_eq!(service_class_name("ßx", &TypeEnv::new()), "SSx");
        assert_eq!(service_class_name("", &TypeEnv::new()), "");
    }

    /// The escape runs after capitalization, which is what turns a basename into a global.
    /// The module imports `Actor` and `Agent`, so a `.did` of that basename would collide.
    #[test]
    fn service_class_name_escapes_names_the_module_occupies() {
        assert_eq!(service_class_name("actor", &TypeEnv::new()), "Actor_");
        assert_eq!(service_class_name("agent", &TypeEnv::new()), "Agent_");
        assert_eq!(
            service_class_name("principal", &TypeEnv::new()),
            "Principal_"
        );
    }

    #[test]
    fn service_class_name_escapes_globals() {
        assert_eq!(service_class_name("map", &TypeEnv::new()), "Map_");
        assert_eq!(service_class_name("set", &TypeEnv::new()), "Set_");
        assert_eq!(service_class_name("error", &TypeEnv::new()), "Error_");
        // Not a reserved word once capitalized, so left alone.
        assert_eq!(service_class_name("class", &TypeEnv::new()), "Class");
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
