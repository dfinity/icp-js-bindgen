use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::*;

/// Module specifier for the generated declarations, as imported by the actor and interface
/// files.
///
/// `service_name` is the `.did` basename, which is also the basename the JavaScript layer
/// writes the declarations file under — so the specifier and the file on disk agree by
/// construction. Anything that transforms the name here (a previous version replaced `-`
/// with `_`) desynchronises the two and emits an import of a module that was never
/// written.
///
/// Note that the `./declarations/` prefix is rewritten away by the JavaScript layer when
/// the `flat` option puts the declarations directly in the output directory; see
/// `flattenImportPath` in `src/core/generate/index.ts`.
pub fn declarations_module_specifier(service_name: &str) -> String {
    format!("./declarations/{service_name}.did")
}

pub fn interface_imports(module: &mut Module, service_name: &str) {
    interface_core_agent_imports(module);
    core_principal_import(module);
    old_bindings_imports_interface(module, service_name);
}

pub fn wrapper_imports(module: &mut Module, service_name: &str) {
    wrapper_core_agent_imports(module);
    core_principal_import(module);
    old_bindings_imports(module, service_name);
}

fn old_bindings_imports_interface(module: &mut Module, service_name: &str) {
    // Import _SERVICE
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("_SERVICE".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            })],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: declarations_module_specifier(service_name).into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        })));
}

fn old_bindings_imports(module: &mut Module, service_name: &str) {
    // Import _SERVICE
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![
                ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: Ident::new("idlFactory".into(), DUMMY_SP, SyntaxContext::empty()),
                    imported: None,
                    is_type_only: false,
                }),
                ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: Ident::new("_SERVICE".into(), DUMMY_SP, SyntaxContext::empty()),
                    imported: None,
                    is_type_only: true,
                }),
            ],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: declarations_module_specifier(service_name).into(),
                raw: None,
            }),
            type_only: false,
            with: None,
            phase: Default::default(),
        })));
}

fn core_principal_import(module: &mut Module) {
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
            span: DUMMY_SP,
            specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("Principal".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: false,
            })],
            src: Box::new(Str {
                span: DUMMY_SP,
                value: "@icp-sdk/core/principal".into(),
                raw: None,
            }),
            type_only: true,
            with: None,
            phase: Default::default(),
        })));
}

fn interface_core_agent_imports(module: &mut Module) {
    let import_decl = ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("HttpAgentOptions".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("ActorConfig".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("Agent".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
        ],
        src: Box::new(Str {
            span: DUMMY_SP,
            value: "@icp-sdk/core/agent".into(),
            raw: None,
        }),
        type_only: false,
        with: None,
        phase: Default::default(),
    };
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
}

fn wrapper_core_agent_imports(module: &mut Module) {
    let import_decl = ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("Actor".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: false,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("HttpAgent".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: false,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("HttpAgentOptions".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("ActorConfig".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("Agent".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: Ident::new("ActorSubclass".into(), DUMMY_SP, SyntaxContext::empty()),
                imported: None,
                is_type_only: true,
            }),
        ],
        src: Box::new(Str {
            span: DUMMY_SP,
            value: "@icp-sdk/core/agent".into(),
            raw: None,
        }),
        type_only: false,
        with: None,
        phase: Default::default(),
    };
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
}
