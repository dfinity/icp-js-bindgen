use super::conversion_functions_generator::TypeConverter;
use super::new_typescript_native_types::{
    add_type_definitions, create_interface_from_service, service_interface_ident,
};
use super::preamble::imports::interface_imports;
use super::preamble::options::interface_options_utils;
use super::utils::EnumDeclarations;
use super::utils::get_ident_guarded;
use super::utils::render_ast;
use crate::bindings::typescript_native::comments::add_comments;
use candid::types::{Type, TypeEnv, TypeInner};
use candid_parser::syntax::{IDLMergedProg, IDLType};
use std::collections::HashMap;
use swc_core::common::DUMMY_SP;
use swc_core::common::Span;
use swc_core::ecma::ast::*;

pub fn compile_interface(
    env: &TypeEnv,
    actor: &Option<Type>,
    service_name: &str,
    prog: &IDLMergedProg,
) -> String {
    let mut enum_declarations: EnumDeclarations = HashMap::new();

    let mut module = Module {
        span: DUMMY_SP,
        body: vec![],
        shebang: None,
    };

    interface_imports(&mut module, service_name);
    interface_options_utils(&mut module);
    let mut comments = swc_core::common::comments::SingleThreadedComments::default();
    let mut cursor = super::comments::PosCursor::new();
    let mut top_level_nodes = (&mut enum_declarations, &mut comments, &mut cursor);
    add_type_definitions(&mut top_level_nodes, env, &mut module, prog);

    let mut actor_module = Module {
        span: DUMMY_SP,
        body: vec![],
        shebang: None,
    };

    if let Some(actor_type) = actor {
        let syntax_actor = prog.resolve_actor().ok().flatten();
        let span = syntax_actor
            .as_ref()
            .map(|s| add_comments(&mut top_level_nodes, s.docs.as_ref()))
            .unwrap_or(DUMMY_SP);

        // Scope the converter to release the mutable borrow of top_level_nodes
        {
            let mut converter = TypeConverter::new(env, &mut top_level_nodes);
            interface_actor_implementation(
                env,
                &mut actor_module,
                actor_type,
                syntax_actor.as_ref().map(|s| &s.typ),
                service_name,
                &mut converter,
                span,
            );

            let mut sorted_functions = converter.get_generated_functions();
            sorted_functions.sort_by(|a, b| {
                if let (Stmt::Decl(Decl::Fn(fn_a)), Stmt::Decl(Decl::Fn(fn_b))) = (a, b) {
                    fn_a.ident.sym.to_string().cmp(&fn_b.ident.sym.to_string())
                } else {
                    std::cmp::Ordering::Equal
                }
            });

            for stmt in sorted_functions {
                actor_module.body.push(ModuleItem::Stmt(stmt.clone()));
            }
        }
    }

    // Add enum declarations to the module, sorted by name for stability
    let mut sorted_enums: Vec<_> = enum_declarations.iter().collect();
    sorted_enums.sort_by_key(|(_, (_, enum_name))| enum_name.clone());

    for (_, enum_decl) in sorted_enums {
        module
            .body
            .push(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                span: DUMMY_SP,
                decl: Decl::TsEnum(Box::new(enum_decl.0.clone())),
            })));
    }

    module.body.extend(actor_module.body);

    // Add CreateActorOptions interface and createActor function declaration if actor exists
    if actor.is_some() {
        add_create_actor_interface_exports(&mut module, service_name);
    }

    // Generate code from the AST
    render_ast(&module, &comments)
}

fn interface_actor_implementation(
    env: &TypeEnv,
    module: &mut Module,
    actor_type: &Type,
    syntax: Option<&IDLType>,
    service_name: &str,
    converter: &mut TypeConverter,
    span: Span,
) {
    match actor_type.as_ref() {
        TypeInner::Service(serv) => {
            interface_actor_service(env, syntax, module, serv, service_name, converter, span)
        }
        TypeInner::Var(id) => interface_actor_var(module, id.as_str(), service_name, span),
        TypeInner::Class(_, t) => {
            if let Some(IDLType::ClassT(_, syntax_t)) = syntax {
                interface_actor_implementation(
                    env,
                    module,
                    t,
                    Some(syntax_t),
                    service_name,
                    converter,
                    span,
                )
            } else {
                interface_actor_implementation(env, module, t, None, service_name, converter, span)
            }
        }
        _ => {}
    }
}

/// Add actor implementation from service definition
pub fn interface_actor_service(
    env: &TypeEnv,
    syntax: Option<&IDLType>,
    module: &mut Module,
    serv: &[(String, Type)],
    service_name: &str,
    converter: &mut TypeConverter,
    span: Span,
) {
    let interface = create_interface_from_service(
        &mut converter.top_level_nodes(),
        env,
        service_name,
        syntax,
        serv,
    );
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            span,
            decl: Decl::TsInterface(Box::new(interface)),
        })));
}

pub fn interface_actor_var(module: &mut Module, type_id: &str, service_name: &str, span: Span) {
    let interface = TsInterfaceDecl {
        span: DUMMY_SP,
        declare: false,
        id: service_interface_ident(service_name),
        type_params: None,
        extends: vec![TsExprWithTypeArgs {
            span: DUMMY_SP,
            expr: Box::new(Expr::Ident(service_interface_ident(type_id))),
            type_args: None,
        }],
        body: TsInterfaceBody {
            span: DUMMY_SP,
            body: vec![],
        },
    };
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            span,
            decl: Decl::TsInterface(Box::new(interface)),
        })));
}

fn add_create_actor_interface_exports(module: &mut Module, service_name: &str) {
    // CreateActorOptions interface
    let create_actor_options_interface = super::preamble::actor::create_actor_options_interface();
    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            span: DUMMY_SP,
            decl: Decl::TsInterface(Box::new(create_actor_options_interface)),
        })));

    // createActor function declaration (no implementation)
    let create_actor_fn_decl = FnDecl {
        ident: get_ident_guarded("createActor"),
        declare: true,
        function: Box::new(swc_core::ecma::ast::Function {
            params: vec![
                Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: get_ident_guarded("canisterId"),
                        type_ann: Some(Box::new(TsTypeAnn {
                            span: DUMMY_SP,
                            type_ann: Box::new(TsType::TsKeywordType(TsKeywordType {
                                span: DUMMY_SP,
                                kind: TsKeywordTypeKind::TsStringKeyword,
                            })),
                        })),
                    }),
                },
                Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Assign(AssignPat {
                        span: DUMMY_SP,
                        left: Box::new(Pat::Ident(BindingIdent {
                            id: get_ident_guarded("options"),
                            type_ann: Some(Box::new(TsTypeAnn {
                                span: DUMMY_SP,
                                type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                                    span: DUMMY_SP,
                                    type_name: TsEntityName::Ident(get_ident_guarded(
                                        "CreateActorOptions",
                                    )),
                                    type_params: None,
                                })),
                            })),
                        })),
                        right: Box::new(Expr::Object(ObjectLit {
                            span: DUMMY_SP,
                            props: vec![],
                        })),
                    }),
                },
            ],
            decorators: vec![],
            span: DUMMY_SP,
            body: None, // No implementation for interface
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: Some(Box::new(TsTypeAnn {
                span: DUMMY_SP,
                type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                    span: DUMMY_SP,
                    type_name: TsEntityName::Ident(service_interface_ident(service_name)),
                    type_params: None,
                })),
            })),
            ctxt: swc_core::common::SyntaxContext::empty(),
        }),
    };

    module
        .body
        .push(ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            span: DUMMY_SP,
            decl: Decl::Fn(create_actor_fn_decl),
        })));
}
