//! Consistency checks on the generated module.
//!
//! The generator emits TypeScript it never re-reads, and both generated files carry
//! `@ts-nocheck`, so an inconsistent module reaches the user's build rather than failing
//! here.
//!
//! These checks cover what can be decided from the module alone: which names it declares and
//! which names it uses. They do *not* check that those names are well-formed identifiers —
//! nothing here would reject `export enum 'Variant_my-tag'`, which is self-consistent and
//! still does not parse. Semantics are the typechecker's job; `tests/typecheck.test.ts` runs
//! `tsc` over the snapshots for that.

use super::utils::{KEYWORDS, OBJECT_PROTOTYPE_NAMES};
use std::collections::HashMap;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};

/// Everything that must hold of a generated module before it is rendered.
pub fn check_module(module: &Module, target: &str) -> Result<(), String> {
    check_representable_methods(module, target)?;
    check_representable_enum_members(module, target)?;
    check_identifiers(module, target)?;
    check_unique_declarations(module, target)?;
    check_type_references(module, target)
}

/// A candid tag named `__proto__` cannot be an enum member.
///
/// TypeScript lowers `enum T { __proto__ = "__proto__" }` to `T["__proto__"] = "__proto__"`,
/// which hits the `Object.prototype` setter and is discarded — `Object.keys(T)` never
/// contains it. Every reference then reads the prototype instead of the tag, so decoding
/// returns an object where a tag was expected and encoding never matches.
///
/// Quoting does not help, because it compiles to the same bracket assignment. Unlike a record
/// key, there is no computed form for an enum member, so the only honest outcome is to refuse.
fn check_representable_enum_members(module: &Module, target: &str) -> Result<(), String> {
    let mut members = EnumMemberNames::default();
    module.visit_with(&mut members);

    match members.names.into_iter().find(|name| name == "__proto__") {
        Some(name) => Err(format!(
            "generated {target} cannot represent the candid variant tag `{name}`: assigning an \
             enum member of that name sets the object's prototype instead of declaring the \
             member, so it would be missing at runtime. Rename the tag in the .did file."
        )),
        None => Ok(()),
    }
}

/// Names the generated wrapper class occupies itself.
///
/// The class body is exactly its constructor plus the candid methods, and it extends nothing
/// — so this is those two names. Adding a member to the class means adding it here.
const CLASS_MEMBER_NAMES: [(&str, &str); 2] = [
    (
        "constructor",
        "a class member of that name is the class constructor, and the spelling that is not \
         overwrites `prototype.constructor`",
    ),
    (
        "actor",
        "the wrapper holds the actor it wraps in a field of that name, which shadows the \
         method at runtime",
    ),
];

/// A candid method whose name the wrapper cannot carry faithfully.
///
/// Two groups. The class occupies `constructor` and `actor` itself, and a method of either
/// name is unreachable — TypeScript rejects the class outright, or the constructor's field
/// assignment wins.
///
/// The rest are `Object.prototype` members, which JavaScript invokes on its own behalf. A
/// method named `toString` is reachable and typechecks, but it overrides a protocol: coercing
/// or logging the actor calls it, so `String(actor)` fires a canister call and then throws
/// `Cannot convert object to primitive value`. `valueOf` and `toLocaleString` do the same, and
/// library code calls `hasOwnProperty` on objects it knows nothing about.
///
/// Refusing those is a judgement rather than a necessity: the method itself would work, and a
/// `.did` naming one becomes ungeneratable. It is the safer default — a canister call should
/// not happen because something interpolated the actor into a string — and it is recorded in
/// `docs/src/content/docs/structure.md` so the limitation is discoverable rather than a
/// surprise.
fn check_representable_methods(module: &Module, target: &str) -> Result<(), String> {
    let mut methods = ClassMethodNames::default();
    module.visit_with(&mut methods);

    for (reserved, reason) in CLASS_MEMBER_NAMES {
        if methods.names.iter().any(|name| name == reserved) {
            return Err(format!(
                "generated {target} cannot represent the candid method `{reserved}`: {reason}. \
                 Rename the method in the .did file."
            ));
        }
    }

    match methods
        .names
        .into_iter()
        .find(|name| OBJECT_PROTOTYPE_NAMES.contains(&name.as_str()))
    {
        Some(name) => Err(format!(
            "generated {target} will not expose the candid method `{name}`: every object \
             inherits a member of that name, and overriding it changes behaviour JavaScript \
             relies on — coercing or logging the actor would call the method, firing a \
             canister call. Rename the method in the .did file."
        )),
        None => Ok(()),
    }
}

/// Every `Ident` the module emits must actually be an identifier.
///
/// The generator builds TypeScript as a typed AST, so the only way it can emit something that
/// does not parse is by putting a name into an `Ident` that cannot be one. A candid name that
/// is not identifier-shaped has to become a string literal or a computed key instead —
/// `candid_prop_name`, `candid_prop_key` and `candid_member_prop` choose between those.
///
/// Checking the finished module rather than the call sites is the point: the recurring defect
/// was a caller deciding its name needed no guarding, and there are far too many `Ident`
/// constructions to keep that judgement correct by inspection.
fn check_identifiers(module: &Module, target: &str) -> Result<(), String> {
    let mut idents = Identifiers::default();
    module.visit_with(&mut idents);

    match idents.names.into_iter().find(|n| !is_valid_ident(n)) {
        Some(name) => Err(format!(
            "generated {target} emits `{name}` as an identifier, which is not one. A candid \
             name that cannot be an identifier belongs in a string literal or a computed key, \
             not in an `Ident`."
        )),
        None => Ok(()),
    }
}

/// Deliberately looser than the generator's own rule, so this asks "would TypeScript accept
/// this" rather than restating the implementation.
fn is_valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => {
            (first == '_' || first == '$' || unicode_ident::is_xid_start(first))
                && chars.all(|c| c == '$' || unicode_ident::is_xid_continue(c))
        }
        None => false,
    }
}

/// No two names may occupy TypeScript's *type* space in the generated module.
///
/// Candid type ids, the `.did` basename and joined variant tags all feed declaration names,
/// and the module also imports names from `@icp-sdk/core` — nothing reconciles them.
///
/// The reason to reject is not that TypeScript refuses these. It mostly does not: an
/// interface merges with a same-named interface, enum, or class without complaint. The result
/// is a type that claims members the runtime value does not have — a wrapper class that
/// silently gains a candid record's fields, or a string enum that accepts members its candid
/// variant never declared, which the conversion functions then put on the wire.
///
/// Value space is excluded. TypeScript merges a function with a same-named interface without
/// error and without inventing members, so a candid type sharing a name with one of the
/// generated helpers is harmless; flagging it would reject working output for no gain.
/// `class` is included because it declares a type as well as a value.
fn check_unique_declarations(module: &Module, target: &str) -> Result<(), String> {
    let mut seen: HashMap<String, &'static str> = HashMap::new();
    for name in imported_names(module) {
        seen.insert(name, "import");
    }

    for (name, kind) in module.body.iter().filter_map(as_decl).filter_map(type_decl) {
        if let Some(previous) = seen.insert(name.clone(), kind) {
            return Err(format!(
                "generated {target} uses the name `{name}` twice (as {previous} and as {kind}). \
                 Candid type names, the .did basename, variant tags and the imports the \
                 generated module needs all feed into this namespace; two of them collided \
                 here. TypeScript would merge them silently rather than reject them, producing \
                 a type that claims members the value does not have."
            ));
        }
    }

    Ok(())
}

/// Every referenced type must resolve to something the module declares or imports.
///
/// A name can reach reference position while its declaration is interned away or escaped
/// differently — the module then references a type it never declares, and consumers cannot
/// import it.
fn check_type_references(module: &Module, target: &str) -> Result<(), String> {
    let mut known: Vec<String> = KEYWORDS.iter().map(|k| k.to_string()).collect();
    known.extend(
        module
            .body
            .iter()
            .filter_map(as_decl)
            .filter_map(declared_name),
    );
    known.extend(imported_names(module));

    let mut references = References::default();
    module.visit_with(&mut references);

    // Type parameters are collected module-wide rather than per-scope. The generated module is
    // flat and its only generics come from the fixed preamble (`Option<T>`, `Some<T>`), so
    // scope tracking would buy nothing; the cost is that a reference is not flagged if some
    // unrelated declaration happens to bind the same name as a type parameter.
    known.extend(references.type_params);

    match references
        .type_refs
        .into_iter()
        .find(|name| !known.contains(name))
    {
        Some(name) => Err(format!(
            "generated {target} references type `{name}`, which it never declares or imports. \
             A declaration was most likely interned away, or escaped to a different name than \
             its references use."
        )),
        None => Ok(()),
    }
}

fn as_decl(item: &ModuleItem) -> Option<&Decl> {
    match item {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl, .. })) => Some(decl),
        ModuleItem::Stmt(Stmt::Decl(decl)) => Some(decl),
        _ => None,
    }
}

/// The name and kind of a declaration that occupies TypeScript's type space.
fn type_decl(decl: &Decl) -> Option<(String, &'static str)> {
    match decl {
        Decl::TsEnum(d) => Some((d.id.sym.to_string(), "enum")),
        Decl::TsInterface(d) => Some((d.id.sym.to_string(), "interface")),
        Decl::Class(d) => Some((d.ident.sym.to_string(), "class")),
        Decl::TsTypeAlias(d) => Some((d.id.sym.to_string(), "type")),
        _ => None,
    }
}

/// The name of any declaration, type space or value space.
fn declared_name(decl: &Decl) -> Option<String> {
    match decl {
        Decl::Fn(d) => Some(d.ident.sym.to_string()),
        _ => type_decl(decl).map(|(name, _)| name),
    }
}

fn imported_names(module: &Module) -> Vec<String> {
    module
        .body
        .iter()
        .filter_map(|item| match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => Some(&import.specifiers),
            _ => None,
        })
        .flatten()
        .map(|specifier| match specifier {
            ImportSpecifier::Named(s) => s.local.sym.to_string(),
            ImportSpecifier::Default(s) => s.local.sym.to_string(),
            ImportSpecifier::Namespace(s) => s.local.sym.to_string(),
        })
        .collect()
}

#[derive(Default)]
struct EnumMemberNames {
    names: Vec<String>,
}

impl Visit for EnumMemberNames {
    fn visit_ts_enum_member(&mut self, node: &TsEnumMember) {
        let name = match &node.id {
            TsEnumMemberId::Ident(ident) => ident.sym.to_string(),
            TsEnumMemberId::Str(s) => s.value.to_string(),
        };
        self.names.push(name);
        node.visit_children_with(self);
    }
}

#[derive(Default)]
struct ClassMethodNames {
    names: Vec<String>,
}

impl Visit for ClassMethodNames {
    fn visit_class_method(&mut self, node: &ClassMethod) {
        let name = match &node.key {
            PropName::Ident(ident) => Some(ident.sym.to_string()),
            PropName::Str(s) => Some(s.value.to_string()),
            _ => None,
        };
        self.names.extend(name);
        node.visit_children_with(self);
    }
}

#[derive(Default)]
struct Identifiers {
    names: Vec<String>,
}

impl Visit for Identifiers {
    fn visit_ident(&mut self, node: &Ident) {
        self.names.push(node.sym.to_string());
        node.visit_children_with(self);
    }

    fn visit_ident_name(&mut self, node: &IdentName) {
        self.names.push(node.sym.to_string());
        node.visit_children_with(self);
    }
}

#[derive(Default)]
struct References {
    type_refs: Vec<String>,
    type_params: Vec<String>,
}

impl Visit for References {
    fn visit_ts_type_ref(&mut self, node: &TsTypeRef) {
        // Only the leftmost segment of `A.B.C` is a binding; the rest are members of it.
        let root = match &node.type_name {
            TsEntityName::Ident(ident) => ident,
            TsEntityName::TsQualifiedName(qualified) => {
                let mut left = &qualified.left;
                loop {
                    match left {
                        TsEntityName::Ident(ident) => break ident,
                        TsEntityName::TsQualifiedName(inner) => left = &inner.left,
                    }
                }
            }
        };
        self.type_refs.push(root.sym.to_string());
        node.visit_children_with(self);
    }

    fn visit_ts_type_param(&mut self, node: &TsTypeParam) {
        self.type_params.push(node.name.sym.to_string());
        node.visit_children_with(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::common::{FileName, SourceMap, sync::Lrc};
    use swc_core::ecma::parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

    /// A function and an interface of the same name merge without error and without inventing
    /// members, so the generator is free to emit that pairing. Pinned because widening the
    /// duplicate check to value space would start rejecting it.
    #[test]
    fn function_beside_a_same_named_interface_is_allowed() {
        let module = parse("export interface Foo { a: string }\nexport function Foo() {}\n");
        assert_eq!(check_unique_declarations(&module, "wrapper"), Ok(()));
    }

    /// A candid type whose name equals the class name derived from the .did basename.
    /// TypeScript merges these, so the class type silently gains the record's fields.
    #[test]
    fn class_colliding_with_an_interface_is_reported() {
        let module =
            parse("export interface My_backend { a: string }\nexport class My_backend {}\n");
        let error = check_unique_declarations(&module, "wrapper").unwrap_err();
        assert!(error.contains("My_backend"), "{error}");
        assert!(error.contains("interface"), "{error}");
        assert!(error.contains("class"), "{error}");
    }

    /// Same-named string enums merge, and the merged enum then accepts members neither candid
    /// variant declared.
    #[test]
    fn enums_that_would_merge_are_reported() {
        let module = parse(concat!(
            "export enum Variant_my_f { 'my-f' = 'my-f' }\n",
            "export enum Variant_my_f { my_f = 'my_f' }\n",
        ));
        let error = check_unique_declarations(&module, "interface").unwrap_err();
        assert!(error.contains("Variant_my_f"), "{error}");
    }

    /// A candid type named after something the module imports shadows it, and the reference
    /// silently means the wrong type.
    #[test]
    fn declaration_colliding_with_an_import_is_reported() {
        let module = parse(concat!(
            "import { type Agent } from '@icp-sdk/core/agent';\n",
            "export interface Agent { b: bigint }\n",
        ));
        let error = check_unique_declarations(&module, "wrapper").unwrap_err();
        assert!(error.contains("Agent"), "{error}");
        assert!(error.contains("import"), "{error}");
    }

    /// A type referenced everywhere and declared nowhere, because its declaration was
    /// interned under another name.
    #[test]
    fn reference_to_an_undeclared_type_is_reported() {
        let module = parse(concat!(
            "export enum RepairStatus { ready = 'ready' }\n",
            "export interface S { status: SoftwareServiceStatus }\n",
        ));
        let error = check_type_references(&module, "interface").unwrap_err();
        assert!(error.contains("SoftwareServiceStatus"), "{error}");
    }

    #[test]
    fn declarations_imports_type_params_and_globals_all_resolve() {
        let module = parse(concat!(
            "import { Principal } from '@icp-sdk/core/principal';\n",
            "export interface Some<T> { value: T }\n",
            "export type Option<T> = Some<T>;\n",
            "export enum Status { ready = 'ready' }\n",
            "export interface S { who: Principal; maybe: Option<Status>; done: Promise<void> }\n",
        ));
        assert_eq!(check_type_references(&module, "interface"), Ok(()));
    }

    /// Fixtures are written as source rather than built as an AST, so a reader can see what is
    /// being asserted. The parser is a dev-dependency only — it is not in the shipped wasm.
    fn parse(source: &str) -> Module {
        let cm: Lrc<SourceMap> = Default::default();
        let fm = cm.new_source_file(
            Lrc::new(FileName::Custom("<test>".to_string())),
            source.to_string(),
        );
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax::default()),
            EsVersion::latest(),
            StringInput::from(&*fm),
            None,
        );
        Parser::new_from(lexer).parse_module().unwrap()
    }
}
