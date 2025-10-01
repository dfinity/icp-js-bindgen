use swc_core::common::{SyntaxContext, DUMMY_SP};
use swc_core::ecma::ast::*;

use crate::bindings::typescript_native::utils::get_ident_guarded;

/// Creates the CreateActorOptions interface used by both wrapper and interface files
pub fn create_actor_options_interface() -> TsInterfaceDecl {
    TsInterfaceDecl {
        span: DUMMY_SP,
        id: "CreateActorOptions".into(),
        declare: false,
        type_params: None,
        extends: vec![],
        body: TsInterfaceBody {
            span: DUMMY_SP,
            body: vec![
                TsTypeElement::TsPropertySignature(TsPropertySignature {
                    span: DUMMY_SP,
                    readonly: false,
                    key: Box::new(Expr::Ident(get_ident_guarded("agent"))),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(get_ident_guarded("Agent")),
                            type_params: None,
                        })),
                    })),
                }),
                TsTypeElement::TsPropertySignature(TsPropertySignature {
                    span: DUMMY_SP,
                    readonly: false,
                    key: Box::new(Expr::Ident(get_ident_guarded("agentOptions"))),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(get_ident_guarded("HttpAgentOptions")),
                            type_params: None,
                        })),
                    })),
                }),
                TsTypeElement::TsPropertySignature(TsPropertySignature {
                    span: DUMMY_SP,
                    readonly: false,
                    key: Box::new(Expr::Ident(get_ident_guarded("actorOptions"))),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(get_ident_guarded("ActorConfig")),
                            type_params: None,
                        })),
                    })),
                }),
            ],
        },
    }
}

pub fn process_error_fn_type() -> TsTypeAliasDecl {
    TsTypeAliasDecl {
        span: DUMMY_SP,
        declare: false,
        id: Ident::new("ProcessErrorFn".into(), DUMMY_SP, SyntaxContext::empty()),
        type_params: None,
        type_ann: Box::new(TsType::TsFnOrConstructorType(
            TsFnOrConstructorType::TsFnType(TsFnType {
                span: DUMMY_SP,
                params: vec![TsFnParam::Ident(BindingIdent {
                    id: Ident::new("error".into(), DUMMY_SP, SyntaxContext::empty()),
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsKeywordType(TsKeywordType {
                            span: DUMMY_SP,
                            kind: TsKeywordTypeKind::TsUnknownKeyword,
                        })),
                    })),
                })],
                type_params: None,
                type_ann: Box::new(TsTypeAnn {
                    span: DUMMY_SP,
                    type_ann: Box::new(TsType::TsKeywordType(TsKeywordType {
                        span: DUMMY_SP,
                        kind: TsKeywordTypeKind::TsNeverKeyword,
                    })),
                }),
            }),
        )),
    }
}