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

use super::utils::KEYWORDS;
use std::collections::HashMap;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};

/// Everything that must hold of a generated module before it is rendered.
pub fn check_module(module: &Module, target: &str) -> Result<(), String> {
    check_distinct_members(module, target)?;
    check_unique_declarations(module, target)?;
    check_type_references(module, target)
}

/// The checks describe the actor files; a caller who wants the declarations alone is not
/// affected by them.
const DECLARATIONS_ONLY_HINT: &str =
    " Generating only the declarations (`output.actor.disabled`) is unaffected.";

/// No object type — type literal, interface or class — may declare the same member twice.
///
/// Field names come from the candid interface, but the generator injects names of its own
/// alongside them — `__kind__`, the discriminant it adds to a variant carrying payloads — and
/// two candid names can escape to one key. The later member silently wins.
fn check_distinct_members(module: &Module, target: &str) -> Result<(), String> {
    let mut members = ObjectMembers::default();
    module.visit_with(&mut members);

    match members.duplicate {
        Some(name) => Err(format!(
            "generated {target} declares `{name}` twice in one object type. The generator adds \
             `__kind__` to a variant that carries payloads, so a candid field or tag of that \
             name collides with it, and two candid names can escape to one key. Rename it in \
             the .did file.{DECLARATIONS_ONLY_HINT}"
        )),
        None => Ok(()),
    }
}

/// No two of the module's top-level bindings may collide.
///
/// Candid type ids, the `.did` basename, joined variant tags, the conversion functions named
/// after the types they convert, and the module's imports all feed into one namespace, and
/// nothing reconciles them. TypeScript mostly merges such collisions rather than rejecting
/// them, yielding a type that claims members the runtime value lacks.
///
/// A declaration occupies TypeScript's type space, its value space, or both, and two of them
/// collide only where they overlap: a `function` and an `interface` of one name are the
/// merge TypeScript intends, while a `function` and an `enum` are two bindings of one name.
fn check_unique_declarations(module: &Module, target: &str) -> Result<(), String> {
    let mut types: HashMap<String, &'static str> = HashMap::new();
    let mut values: HashMap<String, &'static str> = HashMap::new();

    let imports = imported_bindings(module).into_iter();
    let declarations = module.body.iter().filter_map(as_decl).filter_map(binding);

    for (name, kind, spaces) in imports.chain(declarations) {
        for (occupied, space) in [(&mut types, Space::Type), (&mut values, Space::Value)] {
            if !spaces.contains(&space) {
                continue;
            }
            if let Some(previous) = occupied.insert(name.clone(), kind) {
                let kinds = match previous == kind {
                    true => format!("both as {kind}"),
                    false => format!("as {previous} and as {kind}"),
                };
                return Err(format!(
                    "generated {target} uses the name `{name}` twice in {space} space \
                     ({kinds}). Candid type names, the .did basename, variant tags, the \
                     conversion functions and the imports the generated module needs all feed \
                     into this namespace; two of them collided here. TypeScript would merge them \
                     silently rather than reject them, producing a type that claims members the \
                     value does not have. Rename one of them in the .did file; where a side is \
                     the class or the interface named after the file, renaming the file works \
                     too.{DECLARATIONS_ONLY_HINT}"
                ));
            }
        }
    }

    Ok(())
}

/// One of the two namespaces a name can occupy in a TypeScript module.
#[derive(Clone, Copy, PartialEq)]
enum Space {
    Type,
    Value,
}

impl std::fmt::Display for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Space::Type => f.write_str("type"),
            Space::Value => f.write_str("value"),
        }
    }
}

/// Every referenced type must resolve to a *type* the module declares or imports.
///
/// A declaration interned away, or escaped differently from its references, leaves a type
/// consumers cannot import. A binding that lives in value space alone — a function, a value
/// import — does not count: TypeScript would want `typeof` to reach it. A reference to a type
/// *parameter* resolves to the parameter instead, so those are tracked by the scope that binds
/// them rather than module-wide — the preamble's `Option<T>` must not excuse an unrelated `T`
/// elsewhere.
fn check_type_references(module: &Module, target: &str) -> Result<(), String> {
    let mut known: Vec<String> = KEYWORDS.iter().map(|k| k.to_string()).collect();
    let declared = module.body.iter().filter_map(as_decl).filter_map(binding);
    known.extend(
        declared
            .chain(imported_bindings(module))
            .filter(|(_, _, spaces)| spaces.contains(&Space::Type))
            .map(|(name, _, _)| name),
    );

    let mut references = References::default();
    module.visit_with(&mut references);

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

/// The name a declaration binds, its kind for the diagnostic, and the spaces it occupies.
///
/// An `enum` and a `class` declare a type and a value both, which is what lets them collide
/// with a generated function as well as with another type.
fn binding(decl: &Decl) -> Option<(String, &'static str, &'static [Space])> {
    match decl {
        Decl::TsEnum(d) => Some((d.id.sym.to_string(), "enum", BOTH_SPACES)),
        Decl::Class(d) => Some((d.ident.sym.to_string(), "class", BOTH_SPACES)),
        Decl::TsInterface(d) => Some((d.id.sym.to_string(), "interface", &[Space::Type])),
        Decl::TsTypeAlias(d) => Some((d.id.sym.to_string(), "type", &[Space::Type])),
        Decl::Fn(d) => Some((d.ident.sym.to_string(), "function", &[Space::Value])),
        _ => None,
    }
}

const BOTH_SPACES: &[Space] = &[Space::Type, Space::Value];

/// Every local an import binds, and the spaces it binds it in.
///
/// A type-only import binds in type space alone. The module imports the candid shape of each
/// named type, so these locals collide with each other and with the fixed imports.
fn imported_bindings(module: &Module) -> Vec<(String, &'static str, &'static [Space])> {
    module
        .body
        .iter()
        .filter_map(|item| match item {
            ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => Some(import),
            _ => None,
        })
        .flat_map(|import| {
            import.specifiers.iter().map(move |specifier| {
                let (local, type_only) = match specifier {
                    ImportSpecifier::Named(s) => (s.local.sym.to_string(), s.is_type_only),
                    ImportSpecifier::Default(s) => (s.local.sym.to_string(), false),
                    ImportSpecifier::Namespace(s) => (s.local.sym.to_string(), false),
                };
                let spaces: &'static [Space] = match type_only || import.type_only {
                    true => &[Space::Type],
                    false => BOTH_SPACES,
                };
                (local, "import", spaces)
            })
        })
        .collect()
}

/// The first member name declared twice in one object type: a type literal, an interface
/// body or a class body.
#[derive(Default)]
struct ObjectMembers {
    duplicate: Option<String>,
}

impl ObjectMembers {
    fn check(&mut self, names: impl Iterator<Item = Option<String>>) {
        let mut seen: Vec<String> = vec![];
        for name in names.flatten() {
            if seen.contains(&name) && self.duplicate.is_none() {
                self.duplicate = Some(name.clone());
            }
            seen.push(name);
        }
    }
}

impl Visit for ObjectMembers {
    fn visit_ts_type_lit(&mut self, node: &TsTypeLit) {
        self.check(node.members.iter().map(type_element_key));
        node.visit_children_with(self);
    }

    fn visit_ts_interface_body(&mut self, node: &TsInterfaceBody) {
        self.check(node.body.iter().map(type_element_key));
        node.visit_children_with(self);
    }

    fn visit_class(&mut self, node: &Class) {
        self.check(node.body.iter().map(|member| match member {
            ClassMember::Method(method) => prop_name(&method.key),
            ClassMember::ClassProp(prop) => prop_name(&prop.key),
            _ => None,
        }));
        node.visit_children_with(self);
    }
}

fn type_element_key(member: &TsTypeElement) -> Option<String> {
    match member {
        TsTypeElement::TsPropertySignature(prop) => member_key(&prop.key),
        TsTypeElement::TsMethodSignature(method) => member_key(&method.key),
        _ => None,
    }
}

fn prop_name(name: &PropName) -> Option<String> {
    match name {
        PropName::Ident(ident) => Some(ident.sym.to_string()),
        PropName::Str(s) => Some(s.value.to_string()),
        PropName::Computed(_) | PropName::Num(_) | PropName::BigInt(_) => None,
    }
}

/// The candid name a type member was keyed with, if it is spelled out rather than computed.
fn member_key(key: &Expr) -> Option<String> {
    match key {
        Expr::Ident(ident) => Some(ident.sym.to_string()),
        Expr::Lit(Lit::Str(s)) => Some(s.value.to_string()),
        _ => None,
    }
}

#[derive(Default)]
struct References {
    type_refs: Vec<String>,
    /// The type parameters in scope, innermost last. A reference to one of these resolves to
    /// the parameter rather than to a declaration, so it is not a reference to check.
    scopes: Vec<Vec<String>>,
}

impl References {
    fn bound(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.iter().any(|param| param == name))
    }

    /// Visits `node` with `params` in scope.
    fn in_scope<N: VisitWith<Self>>(&mut self, params: Option<&TsTypeParamDecl>, node: &N) {
        let bound: Vec<String> = params
            .map(|decl| {
                decl.params
                    .iter()
                    .map(|param| param.name.sym.to_string())
                    .collect()
            })
            .unwrap_or_default();
        self.scopes.push(bound);
        node.visit_children_with(self);
        self.scopes.pop();
    }
}

impl Visit for References {
    fn visit_ts_type_alias_decl(&mut self, node: &TsTypeAliasDecl) {
        self.in_scope(node.type_params.as_deref(), node);
    }

    fn visit_ts_interface_decl(&mut self, node: &TsInterfaceDecl) {
        self.in_scope(node.type_params.as_deref(), node);
    }

    fn visit_class_decl(&mut self, node: &ClassDecl) {
        self.in_scope(node.class.type_params.as_deref(), node);
    }

    fn visit_function(&mut self, node: &Function) {
        self.in_scope(node.type_params.as_deref(), node);
    }

    fn visit_ts_fn_type(&mut self, node: &TsFnType) {
        self.in_scope(node.type_params.as_deref(), node);
    }

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
        let name = root.sym.to_string();
        if !self.bound(&name) {
            self.type_refs.push(name);
        }
        node.visit_children_with(self);
    }

    /// `interface X extends Y` and `class X implements Y` hold `Y` as an expression rather
    /// than a type reference, so it reaches neither visitor above. The generated interface
    /// extends one when the service is a candid type rather than a literal.
    fn visit_ts_expr_with_type_args(&mut self, node: &TsExprWithTypeArgs) {
        if let Expr::Ident(ident) = &*node.expr {
            self.type_refs.push(ident.sym.to_string());
        }
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

    /// The module imports the service type as `_SERVICE` and each named candid type's shape
    /// under a `_`-prefixed local. Nothing in the module reconciles the two lists, so the
    /// second import of a local would leave the actor typed by whichever one TypeScript took.
    #[test]
    fn duplicate_import_locals_are_reported() {
        let module = parse(concat!(
            "import { idlFactory, type _SERVICE } from './declarations/x.did';\n",
            "import type { SERVICE as _SERVICE } from './declarations/x.did';\n",
        ));
        let error = check_unique_declarations(&module, "wrapper").unwrap_err();
        assert!(error.contains("_SERVICE"), "{error}");
        assert!(error.contains("import"), "{error}");
    }

    /// A conversion function is named after the type it converts, so a candid type can be
    /// named after one. An enum declares a value too, so this is a second binding of the
    /// name rather than the merge a function and an interface would make.
    #[test]
    fn function_colliding_with_an_enum_is_reported() {
        let module = parse(concat!(
            "export enum to_candid_foo_n1 { a = 'a' }\n",
            "function to_candid_foo_n1(value: foo) {}\n",
        ));
        let error = check_unique_declarations(&module, "wrapper").unwrap_err();
        assert!(error.contains("to_candid_foo_n1"), "{error}");
        assert!(error.contains("value space"), "{error}");
    }

    /// A type parameter is only in scope inside the declaration that binds it, so an
    /// unrelated top-level reference to the same name is still a dangling one.
    #[test]
    fn reference_named_after_a_type_parameter_is_reported() {
        let module = parse(concat!(
            "export interface Some<T> { value: T }\n",
            "export interface S { value: T }\n",
        ));
        let error = check_type_references(&module, "wrapper").unwrap_err();
        assert!(error.contains("`T`"), "{error}");
    }

    /// The preamble's generics must not be flagged, which is what the scope is for.
    #[test]
    fn reference_to_a_bound_type_parameter_is_allowed() {
        let module = parse(concat!(
            "export interface Some<T> { value: T }\n",
            "export type Option<T> = Some<T> | None;\n",
            "export interface None {}\n",
            "function some<T>(value: T): Some<T> { return { value }; }\n",
        ));
        assert_eq!(check_type_references(&module, "wrapper"), Ok(()));
    }

    /// The generated interface extends one declared for a candid service type. That target
    /// is a heritage clause rather than a type reference, so it reaches the check by a
    /// different route.
    #[test]
    fn unresolved_heritage_name_is_reported() {
        let module = parse(concat!(
            "export interface TInterface { m(): Promise<void> }\n",
            "export interface xInterface extends SInterface {}\n",
        ));
        let error = check_type_references(&module, "interface").unwrap_err();
        assert!(error.contains("SInterface"), "{error}");
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

    /// Fixtures are TypeScript source rather than hand-built ASTs. The parser is a
    /// dev-dependency and is not in the shipped wasm.
    /// A type reference resolves to a type; a value binding of that name would need `typeof`.
    #[test]
    fn reference_to_a_value_only_binding_is_reported() {
        let module = parse("export function Foo() {}\nexport type Bar = Foo;\n");
        let error = check_type_references(&module, "wrapper").unwrap_err();
        assert!(error.contains("`Foo`"), "{error}");
    }

    #[test]
    fn duplicate_interface_member_is_reported() {
        let module = parse("export interface I { a(): void; a(): void }\n");
        let error = check_distinct_members(&module, "wrapper").unwrap_err();
        assert!(error.contains("`a`"), "{error}");
    }

    #[test]
    fn duplicate_class_method_is_reported() {
        let module = parse("export class C { a() {} a() {} }\n");
        let error = check_distinct_members(&module, "wrapper").unwrap_err();
        assert!(error.contains("`a`"), "{error}");
    }

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
