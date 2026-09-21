use super::comments::PosCursor;
use super::new_typescript_native_types::convert_type_with_converter;
use super::new_typescript_native_types::declaring_type_id;
use super::original_typescript_types::OriginalTypescriptTypes;
use super::utils::{
    EnumDeclarations, OBJECT_PROTOTYPE_NAMES, candid_member_prop, candid_prop_name, resolves_to_opt,
};
use candid::types::{Field, Label, Type, TypeEnv, TypeInner};
use std::collections::{HashMap, HashSet};
use swc_core::common::{DUMMY_SP, SyntaxContext, comments::SingleThreadedComments};
use swc_core::ecma::ast::*;
// Type aliases to simplify complex types used throughout this module

/// Whether every tag of a variant carries `null`, which is what makes it an `enum` rather
/// than a discriminated union.
///
/// An empty variant is excluded: `all(..)` is vacuously true for it, but it has no tags to
/// become members and lowers to `never`, so it is not an enum and has no enum to resolve.
fn is_unit_variant(fields: &[Field]) -> bool {
    !fields.is_empty()
        && fields
            .iter()
            .all(|f| matches!(f.ty.as_ref(), TypeInner::Null))
}

/// `value as never`, for the arm of a decoder that cannot be reached once every tag has been
/// tested with `in`.
///
/// `in` narrows the input away tag by tag, so the arm is already `never` and needs no help —
/// except where a tag is named after an inherited member: the own-property conjunction added
/// for it breaks the narrowing, and the arm is asserted instead.
fn unreachable_value(value: Expr, fields: &[Field]) -> Expr {
    if !fields.iter().any(|f| match &*f.id {
        Label::Named(name) => is_inherited_property(name),
        _ => false,
    }) {
        return value;
    }
    as_never(value)
}

fn as_never(value: Expr) -> Expr {
    Expr::TsAs(TsAsExpr {
        span: DUMMY_SP,
        expr: Box::new(value),
        type_ann: Box::new(TsType::TsKeywordType(TsKeywordType {
            span: DUMMY_SP,
            kind: TsKeywordTypeKind::TsNeverKeyword,
        })),
    })
}

/// Whether `in` would match `name` on any object, making it useless as a tag test.
fn is_inherited_property(name: &str) -> bool {
    OBJECT_PROTOTYPE_NAMES.contains(&name)
}

/// Tests that `object` carries `name` as its own property.
///
/// `in` carries the narrowing TypeScript uses to discriminate the variant union, so it stays.
/// For the names it matches on every object, an own-property test is conjoined — redundant
/// at runtime, since `in` is implied by it, but it preserves the narrowing.
fn has_own_property(object: Expr, name: &str) -> Expr {
    let in_test = Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::In,
        left: Box::new(Expr::Lit(Lit::Str(Str {
            span: DUMMY_SP,
            value: name.into(),
            raw: None,
        }))),
        right: Box::new(object.clone()),
    });

    if !is_inherited_property(name) {
        return in_test;
    }

    Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::LogicalAnd,
        left: Box::new(in_test),
        right: Box::new(has_own_call(object, name)),
    })
}

/// `Object.prototype.hasOwnProperty.call(value, name)` rather than `Object.hasOwn`, which is
/// ES2022: the wrapper is shipped into someone else's build, bundlers do not polyfill
/// built-in methods, and calling a method that does not exist there throws at runtime. Going
/// through `Object.prototype` also survives a decoded object that carries its own
/// `hasOwnProperty` — which is exactly the kind of name this test exists for.
fn has_own_call(object: Expr, name: &str) -> Expr {
    let has_own_property = Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(Ident::new(
                "Object".into(),
                DUMMY_SP,
                SyntaxContext::empty(),
            ))),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "prototype".into(),
            }),
        })),
        prop: MemberProp::Ident(IdentName {
            span: DUMMY_SP,
            sym: "hasOwnProperty".into(),
        }),
    });

    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(has_own_property),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "call".into(),
            }),
        }))),
        args: vec![
            ExprOrSpread {
                spread: None,
                expr: Box::new(object),
            },
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(Str {
                    span: DUMMY_SP,
                    value: name.into(),
                    raw: None,
                }))),
            },
        ],
        type_args: None,
        ctxt: SyntaxContext::empty(),
    })
}

/// `value === undefined || value === null`.
///
/// The counterpart to [`is_present`]. A standalone `opt T` is decoded as `T | null`, but the
/// record path yields `undefined` for an absent field, so a value passed straight from one to
/// the other must read as absent either way.
fn is_absent(value: Expr) -> Expr {
    let equals = |right: Expr| {
        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::EqEqEq,
            left: Box::new(value.clone()),
            right: Box::new(right),
        })
    };
    Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::LogicalOr,
        left: Box::new(equals(Expr::Ident(Ident::new(
            "undefined".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
        )))),
        right: Box::new(equals(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))),
    })
}

/// `value !== undefined && value !== null`.
///
/// Presence, not truthiness: a present `opt nat` of `0`, `opt bool` of `false` or `opt text`
/// of `""` encodes as present.
fn is_present(value: Expr) -> Expr {
    let not_equal = |right: Expr| {
        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::NotEqEq,
            left: Box::new(value.clone()),
            right: Box::new(right),
        })
    };
    Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::LogicalAnd,
        left: Box::new(not_equal(Expr::Ident(Ident::new(
            "undefined".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
        )))),
        right: Box::new(not_equal(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))),
    })
}

/// `value !== undefined`: the test for the *outer* level of a nested optional field, whose
/// declared type carries `null` as a value of its own.
fn is_defined(value: Expr) -> Expr {
    Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::NotEqEq,
        left: Box::new(value),
        right: Box::new(Expr::Ident(Ident::new(
            "undefined".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
        ))),
    })
}

/// `value.length === 0`: candid's absent optional on the wire.
fn is_empty(value: Expr) -> Expr {
    Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::EqEqEq,
        left: Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(value),
            prop: MemberProp::Ident(
                Ident::new("length".into(), DUMMY_SP, SyntaxContext::empty()).into(),
            ),
        })),
        right: Box::new(Expr::Lit(Lit::Num(Number {
            span: DUMMY_SP,
            value: 0.0,
            raw: None,
        }))),
    })
}

/// `value[0]`: the payload of a present optional on the wire.
fn first_element(value: Expr) -> Expr {
    Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(value),
        prop: MemberProp::Computed(ComputedPropName {
            span: DUMMY_SP,
            expr: Box::new(Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value: 0.0,
                raw: None,
            }))),
        }),
    })
}

pub type TopLevelNodes<'a> = (
    &'a mut EnumDeclarations,
    &'a mut SingleThreadedComments,
    &'a mut PosCursor,
);
/// Provides functions to generate TypeScript expressions that convert
/// between new TypeScript Native and original TypeScript (current agent-js) representations by generating conversion functions.
pub struct TypeConverter<'a> {
    env: &'a TypeEnv,
    // Track function names by type
    to_candid_functions: HashMap<Type, String>,
    from_candid_functions: HashMap<Type, String>,
    // Track which types have fully generated functions
    to_candid_generated: HashSet<Type>,
    from_candid_generated: HashSet<Type>,
    // Store all generated function declarations
    generated_functions: HashMap<String, Stmt>,
    // Track types being processed to detect recursion
    processing_to_candid: HashSet<Type>,
    processing_from_candid: HashSet<Type>,
    processing_conversion: HashSet<Type>,
    // Counter for unique function names
    function_counter: usize,
    original_types: OriginalTypescriptTypes<'a>,
    enum_declarations: &'a mut EnumDeclarations,
    // For adding comments to the generated functions
    comments: &'a mut SingleThreadedComments,
    cursor: &'a mut PosCursor,
}

impl<'a> TypeConverter<'a> {
    /// Create a new TypeConverter with the given type environment
    pub fn new(env: &'a TypeEnv, top_level_nodes: &'a mut TopLevelNodes<'a>) -> Self {
        let (enum_declarations, comments, cursor) = top_level_nodes;
        TypeConverter {
            env,
            to_candid_functions: HashMap::new(),
            from_candid_functions: HashMap::new(),
            to_candid_generated: HashSet::new(),
            from_candid_generated: HashSet::new(),
            generated_functions: HashMap::new(),
            processing_to_candid: HashSet::new(),
            processing_from_candid: HashSet::new(),
            processing_conversion: HashSet::new(),
            function_counter: 0,
            original_types: OriginalTypescriptTypes::new(env),
            enum_declarations,
            comments,
            cursor,
        }
    }

    /// Get all generated function declarations
    pub fn get_generated_functions(&self) -> Vec<Stmt> {
        self.generated_functions.values().cloned().collect()
    }

    pub fn top_level_nodes(&mut self) -> TopLevelNodes<'_> {
        (
            &mut self.enum_declarations,
            &mut self.comments,
            &mut self.cursor,
        )
    }

    /// Check if a type requires conversion or can be passed through directly
    fn needs_conversion(&mut self, ty: &Type) -> bool {
        // Check if we already computed this type
        let is_recursive = self.processing_conversion.contains(ty);

        self.processing_conversion.insert(ty.clone());

        if is_recursive {
            return true;
        }

        let result = match ty.as_ref() {
            // Types that don't need conversion
            TypeInner::Null => false,
            TypeInner::Bool => false,
            TypeInner::Text => false,
            TypeInner::Nat8 => false,
            TypeInner::Nat16 => false,
            TypeInner::Nat32 => false,
            TypeInner::Int8 => false,
            TypeInner::Int16 => false,
            TypeInner::Int32 => false,
            TypeInner::Float32 => false,
            TypeInner::Float64 => false,
            TypeInner::Reserved => false,

            TypeInner::Nat => false,
            TypeInner::Int => false,
            TypeInner::Nat64 => false,
            TypeInner::Int64 => false,
            TypeInner::Principal => false,
            TypeInner::Empty => false,
            TypeInner::Func(_) => false,
            TypeInner::Service(_) => false,
            // Types that always need conversion
            TypeInner::Opt(_) => true,
            TypeInner::Variant(_) => true,
            // Container types - need conversion only if their contents need conversion
            TypeInner::Vec(inner) => self.needs_conversion(inner),
            TypeInner::Record(fields) => {
                // Only needs conversion if any field needs conversion
                fields.iter().any(|field| self.needs_conversion(&field.ty))
            }
            TypeInner::Var(id) => {
                // Check if the named type needs conversion
                if let Ok(actual_ty) = self.env.rec_find_type(id) {
                    self.needs_conversion(actual_ty)
                } else {
                    true // Conservative default
                }
            }
            _ => true, // Conservative default - convert if unsure
        };
        self.processing_conversion.remove(ty);
        result
    }

    fn type_prefix(&self, ty: &Type) -> &str {
        match ty.as_ref() {
            TypeInner::Null => "null",
            TypeInner::Bool => "bool",
            TypeInner::Nat => "nat",
            TypeInner::Int => "int",
            TypeInner::Nat8 => "nat8",
            TypeInner::Nat16 => "nat16",
            TypeInner::Nat32 => "nat32",
            TypeInner::Nat64 => "nat64",
            TypeInner::Int8 => "int8",
            TypeInner::Int16 => "int16",
            TypeInner::Int32 => "int32",
            TypeInner::Int64 => "int64",
            TypeInner::Float32 => "float32",
            TypeInner::Float64 => "float64",
            TypeInner::Text => "text",
            TypeInner::Reserved => "reserved",
            TypeInner::Empty => "empty",
            TypeInner::Principal => "principal",
            TypeInner::Opt(_) => "opt",
            TypeInner::Vec(_) => "vec",
            TypeInner::Record(fields) => {
                if self.is_tuple(fields) {
                    "tuple"
                } else {
                    "record"
                }
            }
            TypeInner::Variant(_) => "variant",
            TypeInner::Func(_) => "func",
            TypeInner::Service(_) => "service",
            _ => "anonymous",
        }
    }

    /// Get the function name for converting from TypeScript to Candid
    fn get_to_candid_function_name(&mut self, ty: &Type) -> String {
        if let Some(name) = self.to_candid_functions.get(ty) {
            return name.clone();
        }

        // Generate a base name for the function
        let base_name = match ty.as_ref() {
            TypeInner::Var(id) => format!("to_candid_{}", id),
            _ => {
                // For anonymous types, use a descriptive prefix based on the type
                let type_prefix = self.type_prefix(ty);
                format!("to_candid_{}", type_prefix)
            }
        };

        // Generate a unique name with counter
        self.function_counter += 1;
        let name = format!("{}_n{}", base_name, self.function_counter);

        self.to_candid_functions.insert(ty.clone(), name.clone());
        name
    }

    /// Get the function name for converting from Candid to TypeScript
    fn get_from_candid_function_name(&mut self, ty: &Type) -> String {
        if let Some(name) = self.from_candid_functions.get(ty) {
            return name.clone();
        }

        // Generate a base name for the function
        let base_name = match ty.as_ref() {
            TypeInner::Var(id) => format!("from_candid_{}", id),
            _ => {
                // For anonymous types, use a descriptive prefix based on the type
                let type_prefix = self.type_prefix(ty);
                format!("from_candid_{}", type_prefix)
            }
        };

        // Generate a unique name with counter
        self.function_counter += 1;
        let name = format!("{}_n{}", base_name, self.function_counter);

        self.from_candid_functions.insert(ty.clone(), name.clone());
        name
    }

    /// Create an identifier expression
    fn create_ident(&self, name: &str) -> Expr {
        Expr::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()))
    }

    /// Create a function call expression
    fn create_call(&self, callee: &str, args: Vec<ExprOrSpread>) -> Expr {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(self.create_ident(callee))),
            args,
            type_args: None,
            ctxt: SyntaxContext::empty(),
        })
    }

    /// Create a function call argument
    fn create_arg(&self, expr: Expr) -> ExprOrSpread {
        ExprOrSpread {
            spread: None,
            expr: Box::new(expr),
        }
    }

    /// Generate the body of a TypeScript -> Candid conversion function
    fn generate_to_candid_body(&mut self, ty: &Type, param_name: &str) -> Expr {
        match ty.as_ref() {
            TypeInner::Null => self.create_ident(param_name),
            TypeInner::Bool => self.create_ident(param_name),
            TypeInner::Nat
            | TypeInner::Int
            | TypeInner::Nat64
            | TypeInner::Int64
            | TypeInner::Nat8
            | TypeInner::Nat16
            | TypeInner::Nat32
            | TypeInner::Int8
            | TypeInner::Int16
            | TypeInner::Int32
            | TypeInner::Float32
            | TypeInner::Float64 => self.create_ident(param_name),
            TypeInner::Text => self.create_ident(param_name),
            TypeInner::Reserved => self.create_ident(param_name),
            TypeInner::Empty => {
                // Empty type is represented as null in TypeScript
                Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
            }
            TypeInner::Principal => self.convert_principal_to_candid_body(param_name),
            TypeInner::Opt(inner) => self.convert_opt_to_candid_body(inner, param_name),
            TypeInner::Vec(inner) => self.convert_vec_to_candid_body(inner, param_name),
            TypeInner::Record(fields) => self.convert_record_to_candid_body(fields, param_name),
            TypeInner::Variant(fields) => {
                self.convert_variant_to_candid_body(fields, param_name, None)
            }
            TypeInner::Func(func) => self.convert_func_to_candid_body(func, param_name),
            TypeInner::Service(_) => self.create_ident(param_name), // Pass through as-is
            TypeInner::Var(id) => {
                // For named types, delegate to another conversion function
                if let Ok(actual_ty) = self.env.rec_find_type(id) {
                    // If the actual type doesn't need conversion, return the expression directly
                    if !self.needs_conversion(actual_ty) {
                        return self.create_ident(param_name);
                    }

                    // An all-null variant is lowered to an enum named after *this* candid
                    // type, and TypeScript enums are nominal — so its conversion cannot be
                    // shared with another type that happens to have the same tags. Build the
                    // body here rather than delegating to a structurally-keyed function.
                    if let TypeInner::Variant(variant_fields) = actual_ty.as_ref()
                        && is_unit_variant(variant_fields)
                    {
                        let variant_fields = variant_fields.clone();
                        return self.convert_variant_to_candid_body(
                            &variant_fields,
                            param_name,
                            Some(&self.declaring_type_name(id)),
                        );
                    }

                    // Generate the function for the actual type if needed
                    let inner_function_name = self.get_to_candid_function_name(actual_ty);
                    self.generate_to_candid_function(actual_ty, &inner_function_name);

                    // Return a call to that function
                    self.create_call(
                        &inner_function_name,
                        vec![self.create_arg(self.create_ident(param_name))],
                    )
                } else {
                    // If type not found, pass through unchanged
                    self.create_ident(param_name)
                }
            }
            // For unsupported types, we pass through unchanged
            _ => self.create_ident(param_name),
        }
    }

    // --- Type-specific conversion methods (TypeScript Native -> Candid) ---

    fn convert_principal_to_candid_body(&mut self, param_name: &str) -> Expr {
        // Principal objects are already compatible
        self.create_ident(param_name)
    }

    fn convert_opt_to_candid_body(&mut self, inner: &Type, param_name: &str) -> Expr {
        // For nested options, we need to handle them recursively
        if resolves_to_opt(self.env, inner) {
            // Generate a conversion function for the inner option type
            let inner_function_name = self.get_to_candid_function_name(inner);
            self.generate_to_candid_function(inner, &inner_function_name);

            // Create expression: isNone(value) ? candid_none() : candid_some(inner_fn(unwrap(value)))
            return Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(self.create_ident("isNone"))),
                    args: vec![self.create_arg(self.create_ident(param_name))],
                    type_args: None,
                    ctxt: SyntaxContext::empty(),
                })),
                cons: Box::new(self.create_call("candid_none", vec![])),
                alt: Box::new(self.create_call(
                    "candid_some",
                    vec![self.create_arg(self.create_call(
                        &inner_function_name,
                        vec![self.create_arg(self.create_call(
                            "unwrap",
                            vec![self.create_arg(self.create_ident(param_name))],
                        ))],
                    ))],
                )),
            });
        }

        // For inner types that don't need conversion, we can simplify
        if !self.needs_conversion(inner) {
            return Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(is_absent(self.create_ident(param_name))),
                cons: Box::new(self.create_call("candid_none", vec![])),
                alt: Box::new(self.create_call(
                    "candid_some",
                    vec![self.create_arg(self.create_ident(param_name))],
                )),
            });
        }

        // Only for non-option inner types that need conversion,
        // generate a specific conversion function
        let inner_function_name = self.get_to_candid_function_name(inner);
        self.generate_to_candid_function(inner, &inner_function_name);

        Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: Box::new(is_absent(self.create_ident(param_name))),
            cons: Box::new(self.create_call("candid_none", vec![])),
            alt: Box::new(self.create_call(
                "candid_some",
                vec![self.create_arg(self.create_call(
                    &inner_function_name,
                    vec![self.create_arg(self.create_ident(param_name))],
                ))],
            )),
        })
    }

    fn convert_vec_to_candid_body(&mut self, inner: &Type, param_name: &str) -> Expr {
        // Check if it's a number array that should be converted to a typed array
        match inner.as_ref() {
            TypeInner::Nat8
            | TypeInner::Int8
            | TypeInner::Nat16
            | TypeInner::Int16
            | TypeInner::Nat32
            | TypeInner::Int32
            | TypeInner::Nat64
            | TypeInner::Int64 => {
                // For numeric types, we can just pass through or convert to typed array
                self.create_ident(param_name)
            }
            _ => {
                // Optimization for inner types that don't need conversion
                if !self.needs_conversion(inner) {
                    return self.create_ident(param_name);
                }

                // Get conversion function for inner type
                let inner_function_name = self.get_to_candid_function_name(inner);
                self.generate_to_candid_function(inner, &inner_function_name);

                // value.map(x => to_candid_inner(x))
                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Ident(
                            Ident::new("map".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                        ),
                    }))),
                    args: vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Arrow(ArrowExpr {
                            span: DUMMY_SP,
                            params: vec![Pat::Ident(BindingIdent {
                                id: Ident::new("x".into(), DUMMY_SP, SyntaxContext::empty()),
                                type_ann: None,
                            })],
                            body: Box::new(BlockStmtOrExpr::Expr(Box::new(self.create_call(
                                &inner_function_name,
                                vec![self.create_arg(self.create_ident("x"))],
                            )))),
                            is_async: false,
                            is_generator: false,
                            type_params: None,
                            return_type: None,
                            ctxt: SyntaxContext::empty(),
                        })),
                    }],
                    type_args: None,
                    ctxt: SyntaxContext::empty(),
                })
            }
        }
    }

    fn convert_record_to_candid_body(&mut self, fields: &[Field], param_name: &str) -> Expr {
        // If the record is a tuple, handle differently
        if self.is_tuple(fields) {
            return self.convert_tuple_to_candid_body(fields, param_name);
        }

        // Create a new object with converted fields
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: fields
                .iter()
                .map(|field| {
                    let field_name = match &*field.id {
                        Label::Named(name) => name.clone(),
                        Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
                    };

                    // Get the field from the input object
                    let field_access = Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: candid_member_prop(&field_name),
                    });

                    let prop_name = candid_prop_name(&field_name);

                    // Convert the field value based on its type
                    let value = match field.ty.as_ref() {
                        // A field of `opt X` with `X` itself optional is declared as the
                        // standalone type of `X` — `Cfg | null` for `opt Cfg` — so only
                        // `undefined` is the outer absence; `null` belongs to the inner value
                        // and reaches the standalone converter, which sends it as `[]`.
                        TypeInner::Opt(inner) if resolves_to_opt(self.env, inner) => {
                            let inner_function_name = self.get_to_candid_function_name(inner);
                            self.generate_to_candid_function(inner, &inner_function_name);
                            Expr::Cond(CondExpr {
                                span: DUMMY_SP,
                                test: Box::new(self.field_is_defined(
                                    param_name,
                                    &field_name,
                                    field_access.clone(),
                                )),
                                cons: Box::new(self.create_call(
                                    "candid_some",
                                    vec![self.create_arg(self.create_call(
                                        &inner_function_name,
                                        vec![self.create_arg(field_access.clone())],
                                    ))],
                                )),
                                alt: Box::new(self.create_call("candid_none", vec![])),
                            })
                        }
                        TypeInner::Opt(inner) => {
                            // For optional fields, handle undefined/null specially
                            if !self.needs_conversion(inner) {
                                Expr::Cond(CondExpr {
                                    span: DUMMY_SP,
                                    test: Box::new(self.field_is_present(
                                        param_name,
                                        &field_name,
                                        field_access.clone(),
                                    )),
                                    cons: Box::new(self.create_call(
                                        "candid_some",
                                        vec![self.create_arg(field_access.clone())],
                                    )),
                                    alt: Box::new(self.create_call("candid_none", vec![])),
                                })
                            } else {
                                let inner_function_name = self.get_to_candid_function_name(inner);
                                self.generate_to_candid_function(inner, &inner_function_name);

                                Expr::Cond(CondExpr {
                                    span: DUMMY_SP,
                                    test: Box::new(self.field_is_present(
                                        param_name,
                                        &field_name,
                                        field_access.clone(),
                                    )),
                                    cons: Box::new(self.create_call(
                                        "candid_some",
                                        vec![self.create_arg(self.create_call(
                                            &inner_function_name,
                                            vec![self.create_arg(field_access.clone())],
                                        ))],
                                    )),
                                    alt: Box::new(self.create_call("candid_none", vec![])),
                                })
                            }
                        }
                        _ => {
                            // For normal fields, check if conversion is needed
                            if !self.needs_conversion(&field.ty) {
                                field_access
                            } else {
                                // Convert the value using appropriate function
                                let function_name = self.get_to_candid_function_name(&field.ty);
                                self.generate_to_candid_function(&field.ty, &function_name);
                                self.create_call(
                                    &function_name,
                                    vec![self.create_arg(field_access)],
                                )
                            }
                        }
                    };

                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: prop_name,
                        value: Box::new(value),
                    })))
                })
                .collect(),
        })
    }

    fn convert_tuple_to_candid_body(&mut self, fields: &[Field], param_name: &str) -> Expr {
        // Create a new array with converted fields
        Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    // Access tuple element by index
                    let elem_access = Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: i as f64,
                                raw: None,
                            }))),
                        }),
                    });

                    // Check if conversion is needed
                    let value = if !self.needs_conversion(&field.ty) {
                        elem_access
                    } else {
                        // Convert the tuple element
                        let function_name = self.get_to_candid_function_name(&field.ty);
                        self.generate_to_candid_function(&field.ty, &function_name);
                        self.create_call(&function_name, vec![self.create_arg(elem_access)])
                    };

                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(value),
                    })
                })
                .collect(),
        })
    }

    /// Whether an optional field carries a value.
    ///
    /// For a name every object inherits, reading it off an absent field yields the member from
    /// `Object.prototype` rather than `undefined`, so presence has to be established against
    /// the object's own properties first.
    fn field_is_present(&self, param_name: &str, field_name: &str, access: Expr) -> Expr {
        self.own_field_test(param_name, field_name, is_present(access))
    }

    /// Like [`Self::field_is_present`], but `null` counts as a value: the field's declared
    /// type carries it.
    fn field_is_defined(&self, param_name: &str, field_name: &str, access: Expr) -> Expr {
        self.own_field_test(param_name, field_name, is_defined(access))
    }

    fn own_field_test(&self, param_name: &str, field_name: &str, test: Expr) -> Expr {
        if !is_inherited_property(field_name) {
            return test;
        }
        Expr::Bin(BinExpr {
            span: DUMMY_SP,
            op: BinaryOp::LogicalAnd,
            left: Box::new(has_own_property(self.create_ident(param_name), field_name)),
            right: Box::new(test),
        })
    }

    /// The name of the candid type that actually *declares* this variant.
    ///
    /// Candid resolves `type B = A` transitively, so the id reaching a conversion can be an
    /// alias — and an alias has no enum of its own. Following the chain finds the type the
    /// enum was declared for, which is the only way to pick the right one when another type
    /// happens to share the same tags.
    fn declaring_type_name(&self, id: &str) -> String {
        declaring_type_id(self.env, id)
    }

    fn convert_variant_to_candid_body(
        &mut self,
        fields: &[Field],
        param_name: &str,
        type_name: Option<&str>,
    ) -> Expr {
        // If there are no fields, return the input unchanged
        if fields.is_empty() {
            return self.create_ident(param_name);
        }

        // Check if all variants have the same type (especially null)
        let all_null = fields
            .iter()
            .all(|f| matches!(f.ty.as_ref(), TypeInner::Null));
        if all_null {
            // For enums, compare against enum members
            let enum_name = self.enum_declarations.referenced_name(type_name, fields);

            let mut result = self.create_ident(param_name); // Default fallback

            // Process all fields in reverse order to build the chain
            for field in fields.iter().rev() {
                let field_name = match &*field.id {
                    Label::Named(name) => name.clone(),
                    Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
                };

                let enum_member = Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(self.create_ident(&enum_name)),
                    prop: candid_member_prop(&field_name),
                });

                let condition = Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::EqEq,
                    left: Box::new(self.create_ident(param_name)),
                    right: Box::new(enum_member),
                });

                // Create result: { field_name: null }
                // This key is a candid field name on the wire, so it must be emitted verbatim.
                // Reserved words are valid property names, and escaping one to `new_` would
                // encode a field the candid type does not have.
                let field_result = Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: candid_prop_name(&field_name),
                        value: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                    })))],
                });

                // Create a new conditional with this field's condition and result,
                // with the previous result as the alternate
                result = Expr::Cond(CondExpr {
                    span: DUMMY_SP,
                    test: Box::new(condition),
                    cons: Box::new(field_result),
                    alt: Box::new(result),
                });
            }

            result
        } else {
            // For variants with different types, check for __kind__ property
            // This is for discriminated union: { __kind__: 'tag1', tag1: value1 } | { __kind__: 'tag2', tag2: value2 }

            // Build a series of conditions to check each tag, ending in the arm no tag matched.
            // Testing `__kind__` narrows a union to `never` by then; a variant with one tag is
            // not a union and is never narrowed, so that arm asserts it.
            let mut result = match fields.len() {
                1 => as_never(self.create_ident(param_name)),
                _ => self.create_ident(param_name),
            };

            for field in fields.iter().rev() {
                let field_name = match &*field.id {
                    Label::Named(name) => name.clone(),
                    Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
                };

                // Check if the __kind__ field matches this variant
                let kind_access = Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(self.create_ident(param_name)),
                    prop: MemberProp::Ident(
                        Ident::new("__kind__".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                    ),
                });

                let condition = Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::EqEqEq,
                    left: Box::new(kind_access),
                    right: Box::new(Expr::Lit(Lit::Str(Str {
                        span: DUMMY_SP,
                        value: field_name.clone().into(),
                        raw: None,
                    }))),
                });

                let field_access = Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(self.create_ident(param_name)),
                    prop: candid_member_prop(&field_name),
                });

                let field_result = match field.ty.as_ref() {
                    // A nested optional payload is typed as the standalone `opt X`, so it
                    // converts as one: `None` is `[]`, `Some(null)` is `[[]]`.
                    TypeInner::Opt(inner) if resolves_to_opt(self.env, inner) => {
                        let function_name = self.get_to_candid_function_name(&field.ty);
                        self.generate_to_candid_function(&field.ty, &function_name);
                        self.create_call(
                            &function_name,
                            vec![self.create_arg(field_access.clone())],
                        )
                    }
                    TypeInner::Opt(inner) => {
                        // For optional fields, handle undefined/null specially
                        if !self.needs_conversion(inner) {
                            Expr::Cond(CondExpr {
                                span: DUMMY_SP,
                                test: Box::new(self.field_is_present(
                                    param_name,
                                    &field_name,
                                    field_access.clone(),
                                )),
                                cons: Box::new(self.create_call(
                                    "candid_some",
                                    vec![self.create_arg(field_access.clone())],
                                )),
                                alt: Box::new(self.create_call("candid_none", vec![])),
                            })
                        } else {
                            let inner_function_name = self.get_to_candid_function_name(inner);
                            self.generate_to_candid_function(inner, &inner_function_name);

                            Expr::Cond(CondExpr {
                                span: DUMMY_SP,
                                test: Box::new(self.field_is_present(
                                    param_name,
                                    &field_name,
                                    field_access.clone(),
                                )),
                                cons: Box::new(self.create_call(
                                    "candid_some",
                                    vec![self.create_arg(self.create_call(
                                        &inner_function_name,
                                        vec![self.create_arg(field_access.clone())],
                                    ))],
                                )),
                                alt: Box::new(self.create_call("candid_none", vec![])),
                            })
                        }
                    }
                    _ => {
                        // For normal fields, check if conversion is needed
                        if !self.needs_conversion(&field.ty) {
                            field_access
                        } else {
                            let inner_function_name = self.get_to_candid_function_name(&field.ty);
                            self.generate_to_candid_function(&field.ty, &inner_function_name);
                            // Convert the value using appropriate function
                            self.create_call(
                                &inner_function_name,
                                vec![self.create_arg(field_access.clone())],
                            )
                        }
                    }
                };

                // Create a new conditional with this field's condition and result
                result = Expr::Cond(CondExpr {
                    span: DUMMY_SP,
                    test: Box::new(condition),
                    cons: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: candid_prop_name(&field_name),
                            value: Box::new(field_result),
                        })))],
                    })),
                    alt: Box::new(result),
                });
            }

            result
        }
    }
    fn convert_func_to_candid_body(
        &mut self,
        _func: &candid::types::Function,
        param_name: &str,
    ) -> Expr {
        // Functions are represented as Principals
        self.create_ident(param_name)
    }

    // --- Type-specific conversion methods (Candid -> TypeScript) ---

    /// Generate an expression that converts from Candid to TypeScript representation
    /// for the given Candid type and expression.
    pub fn convert_from_candid(&mut self, expr: &Expr, ty: &Type) -> Expr {
        // For simple types that don't need conversion, return the expression directly
        if !self.needs_conversion(ty) {
            return expr.clone();
        }

        let function_name = self.get_from_candid_function_name(ty);

        // Generate the function if it doesn't exist
        self.generate_from_candid_function(ty, &function_name);

        // Return a call to the function
        self.create_call(&function_name, vec![self.create_arg(expr.clone())])
    }

    /// Generate a function that converts from Candid to TypeScript
    fn generate_from_candid_function(&mut self, ty: &Type, function_name: &str) {
        // Skip if function already generated for this type
        if self.from_candid_generated.contains(ty) {
            return;
        }

        // Check for recursion
        let is_recursive = self.processing_from_candid.contains(ty);

        // Add to processing set to detect recursion
        self.processing_from_candid.insert(ty.clone());

        // Generate parameter type annotation (Candid type)
        let param_name = "value";
        let param_type_ann = self.original_types.get_type(ty);

        // Generate return type annotation (TypeScript type)
        let return_type_ann = self.create_ts_type_annotation(ty);

        // Generate function body
        let body_expr = if is_recursive {
            // For recursive types, return a placeholder initially
            self.create_ident(param_name)
        } else {
            self.generate_from_candid_body(ty, param_name)
        };

        // Create function declaration with type annotations
        let fn_decl = Stmt::Decl(Decl::Fn(FnDecl {
            ident: Ident::new(function_name.into(), DUMMY_SP, SyntaxContext::empty()),
            declare: false,
            function: Box::new(swc_core::ecma::ast::Function {
                params: vec![Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: Ident::new(param_name.into(), DUMMY_SP, SyntaxContext::empty()),
                        type_ann: Some(Box::new(TsTypeAnn {
                            span: DUMMY_SP,
                            type_ann: Box::new(param_type_ann),
                        })),
                    }),
                }],
                decorators: vec![],
                span: DUMMY_SP,
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: vec![Stmt::Return(ReturnStmt {
                        span: DUMMY_SP,
                        arg: Some(Box::new(body_expr)),
                    })],
                    ctxt: SyntaxContext::empty(),
                }),
                is_generator: false,
                is_async: false,
                type_params: None,
                return_type: Some(Box::new(TsTypeAnn {
                    span: DUMMY_SP,
                    type_ann: Box::new(return_type_ann),
                })),
                ctxt: SyntaxContext::empty(),
            }),
        }));

        // Add function to generated list
        self.generated_functions
            .insert(function_name.into(), fn_decl);

        // Mark this type as fully generated
        self.from_candid_generated.insert(ty.clone());

        // If recursive, update the function body now that the function exists
        if is_recursive {
            let body_expr = self.generate_from_candid_body(ty, param_name);
            if let Some(Stmt::Decl(Decl::Fn(fn_decl))) =
                self.generated_functions.get_mut(function_name)
                && let Some(BlockStmt { stmts, .. }) = &mut fn_decl.function.body
                && let Some(Stmt::Return(ret)) = stmts.last_mut()
            {
                ret.arg = Some(Box::new(body_expr));
            }
        }

        // Remove from processing set
        self.processing_from_candid.remove(ty);
    }

    /// Create a TypeScript type annotation for a given Candid type
    fn create_ts_type_annotation(&mut self, ty: &Type) -> TsType {
        convert_type_with_converter(self, self.env, ty, None, true)
    }

    fn generate_from_candid_body(&mut self, ty: &Type, param_name: &str) -> Expr {
        match ty.as_ref() {
            TypeInner::Null => self.create_ident(param_name),
            TypeInner::Bool => self.create_ident(param_name),
            TypeInner::Nat | TypeInner::Int | TypeInner::Nat64 | TypeInner::Int64 => {
                self.convert_from_bigint_body(param_name)
            }
            TypeInner::Nat8
            | TypeInner::Nat16
            | TypeInner::Nat32
            | TypeInner::Int8
            | TypeInner::Int16
            | TypeInner::Int32
            | TypeInner::Float32
            | TypeInner::Float64 => self.create_ident(param_name),
            TypeInner::Text => self.create_ident(param_name),
            TypeInner::Reserved => self.create_ident(param_name),
            TypeInner::Empty => {
                // Empty type is represented as null in JavaScript
                Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
            }
            TypeInner::Principal => self.convert_principal_from_candid_body(param_name),
            TypeInner::Opt(inner) => self.convert_opt_from_candid_body(inner, param_name),
            TypeInner::Vec(inner) => self.convert_vec_from_candid_body(inner, param_name),
            TypeInner::Record(fields) => self.convert_record_from_candid_body(fields, param_name),
            TypeInner::Variant(fields) => {
                self.convert_variant_from_candid_body(fields, param_name, None)
            }
            TypeInner::Func(func) => self.convert_func_from_candid_body(func, param_name),
            TypeInner::Service(_) => self.create_ident(param_name), // Pass through as-is
            TypeInner::Var(id) => {
                // For named types, delegate to another conversion function
                if let Ok(actual_ty) = self.env.rec_find_type(id) {
                    // If the actual type doesn't need conversion, return directly
                    if !self.needs_conversion(actual_ty) {
                        return self.create_ident(param_name);
                    }

                    // An all-null variant is lowered to an enum named after *this* candid
                    // type, and TypeScript enums are nominal — so its conversion cannot be
                    // shared with another type that happens to have the same tags. Build the
                    // body here rather than delegating to a structurally-keyed function.
                    if let TypeInner::Variant(variant_fields) = actual_ty.as_ref()
                        && is_unit_variant(variant_fields)
                    {
                        let variant_fields = variant_fields.clone();
                        return self.convert_variant_from_candid_body(
                            &variant_fields,
                            param_name,
                            Some(&self.declaring_type_name(id)),
                        );
                    }

                    // Generate the function for the actual type if needed
                    let inner_function_name = self.get_from_candid_function_name(actual_ty);
                    self.generate_from_candid_function(actual_ty, &inner_function_name);

                    // Return a call to that function
                    self.create_call(
                        &inner_function_name,
                        vec![self.create_arg(self.create_ident(param_name))],
                    )
                } else {
                    // If type not found, pass through unchanged
                    self.create_ident(param_name)
                }
            }
            // For unsupported types, we pass through unchanged
            _ => self.create_ident(param_name),
        }
    }

    fn convert_from_bigint_body(&mut self, param_name: &str) -> Expr {
        // For now, just pass through bigints
        self.create_ident(param_name)
    }

    fn convert_principal_from_candid_body(&mut self, param_name: &str) -> Expr {
        // Principal objects are already compatible
        self.create_ident(param_name)
    }

    fn convert_opt_from_candid_body(&mut self, inner: &Type, param_name: &str) -> Expr {
        // For inner types that don't need conversion, optimize
        if !self.needs_conversion(inner) {
            return Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::EqEqEq,
                    left: Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Ident(
                            Ident::new("length".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                        ),
                    })),
                    right: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 0.0,
                        raw: None,
                    }))),
                })),
                cons: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                alt: Box::new(Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(self.create_ident(param_name)),
                    prop: MemberProp::Computed(ComputedPropName {
                        span: DUMMY_SP,
                        expr: Box::new(Expr::Lit(Lit::Num(Number {
                            span: DUMMY_SP,
                            value: 0.0,
                            raw: None,
                        }))),
                    }),
                })),
            });
        }

        // Get conversion function for inner type
        let inner_function_name = self.get_from_candid_function_name(inner);
        self.generate_from_candid_function(inner, &inner_function_name);

        if resolves_to_opt(self.env, inner) {
            Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::EqEqEq,
                    left: Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Ident(
                            Ident::new("length".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                        ),
                    })),
                    right: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 0.0,
                        raw: None,
                    }))),
                })),
                // Use null
                cons: Box::new(self.create_call("none", vec![])),
                // Wrap with some()
                alt: Box::new(self.create_call(
                    "some",
                    vec![self.create_arg(self.create_call(
                        &inner_function_name,
                        vec![self.create_arg(Expr::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(self.create_ident(param_name)),
                            prop: MemberProp::Computed(ComputedPropName {
                                span: DUMMY_SP,
                                expr: Box::new(Expr::Lit(Lit::Num(Number {
                                    span: DUMMY_SP,
                                    value: 0.0,
                                    raw: None,
                                }))),
                            }),
                        }))],
                    ))],
                )),
            })
        } else {
            Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(Expr::Bin(BinExpr {
                    span: DUMMY_SP,
                    op: BinaryOp::EqEqEq,
                    left: Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Ident(
                            Ident::new("length".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                        ),
                    })),
                    right: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 0.0,
                        raw: None,
                    }))),
                })),
                // Use null
                cons: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                // Call conversion function
                alt: Box::new(self.create_call(
                    &inner_function_name,
                    vec![self.create_arg(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: 0.0,
                                raw: None,
                            }))),
                        }),
                    }))],
                )),
            })
        }
    }
    fn convert_vec_from_candid_body(&mut self, inner: &Type, param_name: &str) -> Expr {
        // Check if it's a typed array that should be converted to a regular array
        match inner.as_ref() {
            TypeInner::Nat8
            | TypeInner::Int8
            | TypeInner::Nat16
            | TypeInner::Int16
            | TypeInner::Nat32
            | TypeInner::Int32
            | TypeInner::Nat64
            | TypeInner::Int64 => {
                // For numeric types, convert from TypedArray to Array using Array.from
                self.create_call(
                    "Array.from",
                    vec![self.create_arg(self.create_ident(param_name))],
                )
            }
            _ => {
                // Optimization for inner types that don't need conversion
                if !self.needs_conversion(inner) {
                    return self.create_ident(param_name);
                }

                // Get conversion function for inner type
                let inner_function_name = self.get_from_candid_function_name(inner);
                self.generate_from_candid_function(inner, &inner_function_name);

                // value.map(x => from_candid_inner(x))
                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Ident(
                            Ident::new("map".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                        ),
                    }))),
                    args: vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Arrow(ArrowExpr {
                            span: DUMMY_SP,
                            params: vec![Pat::Ident(BindingIdent {
                                id: Ident::new("x".into(), DUMMY_SP, SyntaxContext::empty()),
                                type_ann: None,
                            })],
                            body: Box::new(BlockStmtOrExpr::Expr(Box::new(self.create_call(
                                &inner_function_name,
                                vec![self.create_arg(self.create_ident("x"))],
                            )))),
                            is_async: false,
                            is_generator: false,
                            type_params: None,
                            return_type: None,
                            ctxt: SyntaxContext::empty(),
                        })),
                    }],
                    type_args: None,
                    ctxt: SyntaxContext::empty(),
                })
            }
        }
    }

    fn convert_record_from_candid_body(&mut self, fields: &[Field], param_name: &str) -> Expr {
        // If the record is a tuple, handle differently
        if self.is_tuple(fields) {
            return self.convert_tuple_from_candid_body(fields, param_name);
        }

        // Create a new object with converted fields
        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: fields
                .iter()
                .map(|field| {
                    let field_name = match &*field.id {
                        Label::Named(name) => name.clone(),
                        Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
                    };

                    // Get the field from the input object
                    let field_access = Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: candid_member_prop(&field_name),
                    });

                    let prop_name = candid_prop_name(&field_name);

                    // Convert the field value based on its type
                    let value = match field.ty.as_ref() {
                        // The declared field is the standalone type of the inner `X`, so the
                        // outer level alone becomes `undefined` and the payload converts as a
                        // standalone `X`: `[[]]` reads as `null`, `[[v]]` as `v`.
                        TypeInner::Opt(inner) if resolves_to_opt(self.env, inner) => {
                            let inner_function_name = self.get_from_candid_function_name(inner);
                            self.generate_from_candid_function(inner, &inner_function_name);
                            Expr::Cond(CondExpr {
                                span: DUMMY_SP,
                                test: Box::new(is_empty(field_access.clone())),
                                cons: Box::new(self.create_ident("undefined")),
                                alt: Box::new(self.create_call(
                                    &inner_function_name,
                                    vec![self.create_arg(first_element(field_access.clone()))],
                                )),
                            })
                        }
                        TypeInner::Opt(_) => {
                            // For optional fields, use a utility function
                            Expr::Call(CallExpr {
                                span: DUMMY_SP,
                                callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                                    "record_opt_to_undefined".into(),
                                    DUMMY_SP,
                                    SyntaxContext::empty(),
                                )))),
                                args: vec![ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(
                                        self.convert_from_candid(&field_access, &field.ty),
                                    ),
                                }],
                                type_args: None,
                                ctxt: SyntaxContext::empty(),
                            })
                        }
                        _ => {
                            // For normal fields, check if conversion is needed
                            if !self.needs_conversion(&field.ty) {
                                field_access
                            } else {
                                // Convert the value using appropriate function
                                let function_name = self.get_from_candid_function_name(&field.ty);
                                self.generate_from_candid_function(&field.ty, &function_name);
                                self.create_call(
                                    &function_name,
                                    vec![self.create_arg(field_access)],
                                )
                            }
                        }
                    };

                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: prop_name,
                        value: Box::new(value),
                    })))
                })
                .collect(),
        })
    }

    fn convert_tuple_from_candid_body(&mut self, fields: &[Field], param_name: &str) -> Expr {
        // Create a new array with converted fields
        Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    // Access tuple element by index
                    let elem_access = Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(self.create_ident(param_name)),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: i as f64,
                                raw: None,
                            }))),
                        }),
                    });

                    // Check if conversion is needed
                    let value = if !self.needs_conversion(&field.ty) {
                        elem_access
                    } else {
                        // Convert the tuple element
                        let function_name = self.get_from_candid_function_name(&field.ty);
                        self.generate_from_candid_function(&field.ty, &function_name);
                        self.create_call(&function_name, vec![self.create_arg(elem_access)])
                    };

                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(value),
                    })
                })
                .collect(),
        })
    }

    fn convert_variant_from_candid_body(
        &mut self,
        fields: &[Field],
        param_name: &str,
        type_name: Option<&str>,
    ) -> Expr {
        if fields.is_empty() {
            return self.create_ident(param_name);
        }
        // Check if all variants have the same type (especially null)
        let all_null = fields
            .iter()
            .all(|f| matches!(f.ty.as_ref(), TypeInner::Null));

        // For variants with all null or same simple type, return the enum member
        if all_null {
            // Determine the enum name based on whether this is a named type or anonymous
            let enum_name = self.enum_declarations.referenced_name(type_name, fields);

            let mut conditions = Vec::new();

            for field in fields {
                let field_name = match &*field.id {
                    Label::Named(name) => name.clone(),
                    Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
                };

                // Check if this field exists in the input object
                let test = has_own_property(self.create_ident(param_name), &field_name);

                // Return the enum member access
                // let result =
                let result = Expr::Member(MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(self.create_ident(&enum_name)),
                    prop: candid_member_prop(&field_name),
                });

                conditions.push((test, result));
            }

            // Build the chain of conditionals
            if conditions.is_empty() {
                // Return the input unchanged if no fields (shouldn't happen for valid variants)
                return self.create_ident(param_name);
            }

            // Start with the last condition and build in reverse
            let mut pairs = conditions.into_iter().rev();

            // Get the last pair
            let (last_test, last_result) = match pairs.next() {
                Some(pair) => pair,
                None => return self.create_ident(param_name), // Shouldn't happen due to the check above
            };

            // Build the chain
            let mut expr = Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(last_test),
                cons: Box::new(last_result),
                // Unreachable for a well-formed candid value: every tag is covered above. The
                // assertion says so, since an own-property test does not narrow the union away the
                // way `in` did.
                alt: Box::new(unreachable_value(self.create_ident(param_name), fields)),
            });

            // Add the rest of the conditions
            for (test, result) in pairs {
                expr = Expr::Cond(CondExpr {
                    span: DUMMY_SP,
                    test: Box::new(test),
                    cons: Box::new(result),
                    alt: Box::new(expr),
                });
            }

            return expr;
        }

        // For variants with different types, create discriminated union objects
        let mut conditions = Vec::new();

        for field in fields {
            let field_name = match &*field.id {
                Label::Named(name) => name.clone(),
                Label::Id(n) | Label::Unnamed(n) => format!("_{}_", n),
            };

            // Check if this field exists in the input object
            let test = has_own_property(self.create_ident(param_name), &field_name);

            // Get the field value
            let field_access = Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.create_ident(param_name)),
                prop: candid_member_prop(&field_name),
            });

            // Convert the field value if needed
            let value = if !self.needs_conversion(&field.ty) {
                field_access
            } else {
                let function_name = self.get_from_candid_function_name(&field.ty);
                self.generate_from_candid_function(&field.ty, &function_name);
                self.create_call(&function_name, vec![self.create_arg(field_access)])
            };

            // Create a discriminated union object with __kind__ and the field value
            let result = Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![
                    // __kind__ property
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Ident(
                            Ident::new("__kind__".into(), DUMMY_SP, SyntaxContext::empty()).into(),
                        ),
                        value: Box::new(Expr::Lit(Lit::Str(Str {
                            span: DUMMY_SP,
                            value: field_name.clone().into(),
                            raw: None,
                        }))),
                    }))),
                    // field value property
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: candid_prop_name(&field_name),
                        value: Box::new(value),
                    }))),
                ],
            });

            conditions.push((test, result));
        }

        // Build the chain of conditionals
        if conditions.is_empty() {
            // Return the input unchanged if no fields (shouldn't happen for valid variants)
            return self.create_ident(param_name);
        }

        // Start with the last condition and build in reverse
        let mut pairs = conditions.into_iter().rev();

        // Get the last pair
        let (last_test, last_result) = match pairs.next() {
            Some(pair) => pair,
            None => return self.create_ident(param_name), // Shouldn't happen due to the check above
        };

        // Build the chain
        let mut expr = Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: Box::new(last_test),
            cons: Box::new(last_result),
            // Unreachable for a well-formed candid value: every tag is covered above. The
            // assertion says so, since an own-property test does not narrow the union away the
            // way `in` did.
            alt: Box::new(unreachable_value(self.create_ident(param_name), fields)),
        });

        // Add the rest of the conditions
        for (test, result) in pairs {
            expr = Expr::Cond(CondExpr {
                span: DUMMY_SP,
                test: Box::new(test),
                cons: Box::new(result),
                alt: Box::new(expr),
            });
        }

        expr
    }
    fn convert_func_from_candid_body(
        &mut self,
        _func: &candid::types::Function,
        param_name: &str,
    ) -> Expr {
        // Functions are represented as Principals
        self.create_ident(param_name)
    }

    // Helper to detect tuple types
    fn is_tuple(&self, fields: &[Field]) -> bool {
        if fields.is_empty() {
            return false;
        }
        for (i, field) in fields.iter().enumerate() {
            if field.id.get_id() != (i as u32) {
                return false;
            }
        }
        true
    }

    pub fn convert_to_candid(&mut self, expr: &Expr, ty: &Type) -> Expr {
        // For simple types that don't need conversion, return the expression directly
        if !self.needs_conversion(ty) {
            return expr.clone();
        }

        let function_name = self.get_to_candid_function_name(ty);

        // Generate the function if it doesn't exist
        self.generate_to_candid_function(ty, &function_name);

        // Return a call to the function
        self.create_call(&function_name, vec![self.create_arg(expr.clone())])
    }

    /// Add imports for Candid types
    pub fn add_import_for_original_type_definitions(
        &mut self,
        module: &mut Module,
        service_name: &str,
    ) {
        self.original_types
            .add_import_for_original_type_definitions(module, service_name);
    }

    /// Generate a function that converts from TypeScript to Candid
    fn generate_to_candid_function(&mut self, ty: &Type, function_name: &str) {
        // Skip if function already generated for this type
        if self.to_candid_generated.contains(ty) {
            return;
        }

        // Check for recursion
        let is_recursive = self.processing_to_candid.contains(ty);

        // Add to processing set to detect recursion
        self.processing_to_candid.insert(ty.clone());

        // Generate parameter type annotation
        let param_name = "value";
        let param_type_ann = self.create_ts_type_annotation(ty);

        // Generate return type annotation - use Candid type for complex types
        let return_type_ann = self.original_types.get_type(ty);

        // Generate function body
        let body_expr = if is_recursive {
            // For recursive types, return a placeholder initially
            self.create_ident(param_name)
        } else {
            self.generate_to_candid_body(ty, param_name)
        };

        // Create function declaration with type annotations
        let fn_decl = Stmt::Decl(Decl::Fn(FnDecl {
            ident: Ident::new(function_name.into(), DUMMY_SP, SyntaxContext::empty()),
            declare: false,
            function: Box::new(swc_core::ecma::ast::Function {
                ctxt: SyntaxContext::empty(),
                params: vec![Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: Pat::Ident(BindingIdent {
                        id: Ident::new(param_name.into(), DUMMY_SP, SyntaxContext::empty()),
                        type_ann: Some(Box::new(TsTypeAnn {
                            span: DUMMY_SP,
                            type_ann: Box::new(param_type_ann),
                        })),
                    }),
                }],
                decorators: vec![],
                span: DUMMY_SP,
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: vec![Stmt::Return(ReturnStmt {
                        span: DUMMY_SP,
                        arg: Some(Box::new(body_expr)),
                    })],
                    ctxt: SyntaxContext::empty(),
                }),
                is_generator: false,
                is_async: false,
                type_params: None,
                return_type: Some(Box::new(TsTypeAnn {
                    span: DUMMY_SP,
                    type_ann: Box::new(return_type_ann),
                })),
            }),
        }));

        self.generated_functions
            .insert(function_name.into(), fn_decl);

        // Mark this type as fully generated
        self.to_candid_generated.insert(ty.clone());

        // If recursive, update the function body now that the function exists
        if is_recursive {
            let body_expr = self.generate_to_candid_body(ty, param_name);
            if let Some(Stmt::Decl(Decl::Fn(fn_decl))) =
                self.generated_functions.get_mut(function_name)
                && let Some(BlockStmt { stmts, .. }) = &mut fn_decl.function.body
                && let Some(Stmt::Return(ret)) = stmts.last_mut()
            {
                ret.arg = Some(Box::new(body_expr));
            }
        }

        // Remove from processing set
        self.processing_to_candid.remove(ty);
    }
}

pub fn convert_multi_return_from_candid(
    converter: &mut TypeConverter,
    expr: &Expr,
    types: &[Type],
) -> Expr {
    if types.is_empty() {
        // No return value, return void
        Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
    } else if types.len() == 1 {
        // Single return value
        converter.convert_from_candid(expr, &types[0])
    } else {
        // Multiple return values in a tuple
        Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: types
                .iter()
                .enumerate()
                .map(|(i, ty)| {
                    // Access return value by index
                    let elem_expr = Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(expr.clone()),
                        prop: MemberProp::Computed(ComputedPropName {
                            span: DUMMY_SP,
                            expr: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: i as f64,
                                raw: None,
                            }))),
                        }),
                    });

                    // If type doesn't need conversion, use it directly
                    let value = if !converter.needs_conversion(ty) {
                        elem_expr
                    } else {
                        // Convert the return value using the appropriate function
                        let function_name = converter.get_from_candid_function_name(ty);
                        converter.generate_from_candid_function(ty, &function_name);
                        Expr::Call(CallExpr {
                            span: DUMMY_SP,
                            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                                function_name.into(),
                                DUMMY_SP,
                                SyntaxContext::empty(),
                            )))),
                            args: vec![ExprOrSpread {
                                spread: None,
                                expr: Box::new(elem_expr),
                            }],
                            type_args: None,
                            ctxt: SyntaxContext::empty(),
                        })
                    };

                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(value),
                    })
                })
                .collect(),
        })
    }
}
