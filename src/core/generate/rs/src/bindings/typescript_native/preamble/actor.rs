use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::*;

use crate::bindings::typescript_native::utils::get_ident;

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
                    key: Box::new(Expr::Ident(get_ident("agent"))),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(get_ident("Agent")),
                            type_params: None,
                        })),
                    })),
                }),
                TsTypeElement::TsPropertySignature(TsPropertySignature {
                    span: DUMMY_SP,
                    readonly: false,
                    key: Box::new(Expr::Ident(get_ident("agentOptions"))),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(get_ident("HttpAgentOptions")),
                            type_params: None,
                        })),
                    })),
                }),
                TsTypeElement::TsPropertySignature(TsPropertySignature {
                    span: DUMMY_SP,
                    readonly: false,
                    key: Box::new(Expr::Ident(get_ident("actorOptions"))),
                    computed: false,
                    optional: true,
                    type_ann: Some(Box::new(TsTypeAnn {
                        span: DUMMY_SP,
                        type_ann: Box::new(TsType::TsTypeRef(TsTypeRef {
                            span: DUMMY_SP,
                            type_name: TsEntityName::Ident(get_ident("ActorConfig")),
                            type_params: None,
                        })),
                    })),
                }),
            ],
        },
    }
}
