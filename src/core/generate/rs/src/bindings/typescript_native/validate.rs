//! Consistency checks on the generated module.
//!
//! The generator emits TypeScript it never re-reads, and both generated files carry
//! `@ts-nocheck`, so an inconsistent module reaches the user's build rather than failing
//! here.
//!
//! These checks cover what can be decided from the module alone: which names it declares and
//! which names it uses. Semantics are the typechecker's job — `tests/typecheck.test.ts` runs
//! `tsc` over the snapshots for that.

use super::utils::KEYWORDS;
use std::collections::HashMap;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{Visit, VisitWith};

/// Everything that must hold of a generated module before it is rendered.
pub fn check_module(module: &Module, target: &str) -> Result<(), String> {
    check_unique_declarations(module, target)?;
    check_type_references(module, target)
}

/// No two declarations may claim the same name in TypeScript's *type* space.
///
/// Three independent sources feed declaration names — candid type ids, the `.did` basename,
/// and variant tags joined into `Variant_a_b` — and nothing reconciles them. `type Map` beside
/// `type Map_` both escape to `Map_`; a candid type named like the service collides with the
/// actor class; two variants whose tags sanitize alike both declare one enum.
///
/// None of those are reliably caught downstream, because TypeScript *merges* same-named
/// interfaces and same-named string enums rather than rejecting them. The merged type then
/// admits members the candid type does not have, and the wrapper encodes them as-is.
///
/// Only type space is checked. The generated `to_candid_*`/`from_candid_*` functions and
/// `createActor` live in value space, where TypeScript legally merges a function with a
/// same-named interface — flagging those would reject valid output. `class` counts, because a
/// class declares a type *and* a value, so it collides with an interface rather than merging.
fn check_unique_declarations(module: &Module, target: &str) -> Result<(), String> {
    let mut seen: HashMap<String, &'static str> = HashMap::new();

    for (name, kind) in module.body.iter().filter_map(as_decl).filter_map(type_decl) {
        if let Some(previous) = seen.insert(name.clone(), kind) {
            let kinds = if previous == kind {
                format!("both as {kind}")
            } else {
                format!("as {previous} and as {kind}")
            };
            return Err(format!(
                "generated {target} declares `{name}` twice ({kinds}). Candid type names, the \
                 .did basename and variant tags all feed generated declaration names; two of \
                 them collided here. Rename one of the candid types, or the .did file."
            ));
        }
    }

    Ok(())
}

/// Every referenced type must resolve to something the module declares or imports.
///
/// A name can reach reference position while its declaration is interned away or escaped
/// differently — the module then references a type it never declares, and consumers cannot
/// import it. Two candid types with identical variant tags, for instance, share one enum
/// declaration, so the second is referenced everywhere and declared nowhere.
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

    /// A function and an interface of the same name merge legally in TypeScript, and the
    /// generator emits exactly that pairing — `to_candid_X` beside a type `X`. Pinned because
    /// widening the duplicate check to value space would reject every generated module.
    #[test]
    fn function_beside_a_same_named_interface_is_allowed() {
        let module = parse("export interface Foo { a: string }\nexport function Foo() {}\n");
        assert_eq!(check_unique_declarations(&module, "wrapper"), Ok(()));
    }

    /// A candid type whose name equals the class name derived from the .did basename.
    #[test]
    fn class_colliding_with_an_interface_is_reported() {
        let module =
            parse("export interface My_backend { a: string }\nexport class My_backend {}\n");
        let error = check_unique_declarations(&module, "wrapper").unwrap_err();
        assert!(error.contains("My_backend"), "{error}");
        assert!(error.contains("interface"), "{error}");
        assert!(error.contains("class"), "{error}");
    }

    /// Two variants whose tags sanitize to the same name. TypeScript merges same-named string
    /// enums silently, so nothing downstream would report this.
    #[test]
    fn enums_that_would_merge_silently_are_reported() {
        let module = parse(concat!(
            "export enum Variant_my_f { 'my-f' = 'my-f' }\n",
            "export enum Variant_my_f { my_f = 'my_f' }\n",
        ));
        let error = check_unique_declarations(&module, "interface").unwrap_err();
        assert!(error.contains("Variant_my_f"), "{error}");
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
