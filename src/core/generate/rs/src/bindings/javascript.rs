//! Ported from https://github.com/dfinity/candid/blob/1ddf879f368f765145223c08bbe2c8c8f4782dcc/rust/candid_parser/src/bindings/javascript.rs

use candid::pretty::candid::pp_mode;
use candid::pretty::utils::*;
use candid::types::{Field, Function, Label, SharedLabel, Type, TypeEnv, TypeInner};
use candid_parser::bindings::analysis::{chase_actor, chase_types, infer_rec};
use candid_parser::syntax::IDLMergedProg;
use pretty::RcDoc;
use std::collections::BTreeSet;

// The definition of tuple is language specific.
pub fn is_tuple(t: &Type) -> bool {
    match t.as_ref() {
        TypeInner::Record(fs) => is_tuple_fields(fs),
        _ => false,
    }
}

pub(crate) fn is_tuple_fields(fs: &[Field]) -> bool {
    if fs.is_empty() {
        return false;
    }
    for (i, field) in fs.iter().enumerate() {
        if field.id.get_id() != (i as u32) {
            return false;
        }
    }
    true
}

pub(crate) static KEYWORDS: [&str; 64] = [
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
];
/// Every identifier the generated declarations files write that is not a candid type: what
/// they import, what they export, and the built-in types the `.did.d.ts` printer references.
/// A candid type of one of these names is escaped, since inside the file it would be the
/// thing every such reference resolves to.
///
/// The JavaScript factory imports `IDL` *and* binds it as its parameter, so a candid type of
/// that name produced `const IDL = IDL.Record(…)` — a `SyntaxError`, not just a shadow. The
/// TypeScript declarations import `Principal` and `ActorMethod`, where a type of either name
/// is a duplicate identifier and every reference to it silently means the wrong type. The
/// exports collide with themselves: `_SERVICE` merges with a same-named interface, and a
/// second `export const idlFactory` does not parse. And a `vec` is printed as `Array<…>` or
/// as the typed array for its element, so a candid type named `Uint8Array` would type every
/// `vec nat8` in the file as that record.
///
/// The type printers are closed over candid's constructors, so this list is complete. The
/// tests in this module and in `typescript.rs` render every constructor through the type
/// printers and check the built-ins; the imports and exports are pinned by the
/// `declaration_globals` fixture.
pub(crate) static DECLARATIONS_MODULE_NAMES: [&str; 17] = [
    "IDL",
    "Principal",
    "ActorMethod",
    "_SERVICE",
    "idlFactory",
    "init",
    "idlService",
    "idlInitArgs",
    "Array",
    "Uint8Array",
    "Uint16Array",
    "Uint32Array",
    "BigUint64Array",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "BigInt64Array",
];

/// Names TypeScript accepts as identifiers but not as a type: the intrinsic types, which it
/// refuses as a declaration name (`export interface never { … }` is `TS2427`); `as`, which it
/// refuses as an alias name; and the type operators `keyof`, `readonly`, `infer` and
/// `unique`, which declare fine but parse as the operator wherever the type is referenced.
/// The reserved words in `KEYWORDS` cover the rest.
static TYPESCRIPT_RESERVED_TYPE_NAMES: [&str; 14] = [
    "any",
    "as",
    "bigint",
    "infer",
    "keyof",
    "never",
    "number",
    "object",
    "readonly",
    "string",
    "symbol",
    "undefined",
    "unique",
    "unknown",
];

/// Refuses two candid types that escape to one exported name, e.g. `IDL` and `IDL_`.
///
/// The declarations files are printed rather than built as an AST, so nothing downstream
/// sees the duplicate: TypeScript merges the two interfaces, and every reference to either
/// type resolves to the merged one.
pub(crate) fn check_declaration_names(env: &TypeEnv) -> Result<(), String> {
    let mut seen: Vec<(String, &str)> = Vec::new();
    for (id, _) in env.0.iter() {
        let escaped = escaped_ident_name(id);
        if let Some((_, previous)) = seen.iter().find(|(name, _)| *name == escaped) {
            return Err(format!(
                "candid types `{previous}` and `{id}` would both be exported as `{escaped}` \
                 from the generated declarations, where TypeScript merges them silently. \
                 Rename one of them in the .did file."
            ));
        }
        seen.push((escaped, id.as_str()));
    }
    Ok(())
}

/// The naming rule for identifiers exported by the generated declarations files.
///
/// Anything importing from those files must escape names with this same rule, or the
/// import will name a member that does not exist. Note the keyword list is deliberately
/// narrower than `typescript_native::utils::KEYWORDS`, which also covers TypeScript built-in
/// type names — escaping an import with that wider list would over-escape.
///
/// Applied to candid *type* names only. Field and method names are quoted keys, which
/// collide with nothing.
pub(crate) fn escaped_ident_name(id: &str) -> String {
    if KEYWORDS.contains(&id)
        || DECLARATIONS_MODULE_NAMES.contains(&id)
        || TYPESCRIPT_RESERVED_TYPE_NAMES.contains(&id)
    {
        format!("{}_", id)
    } else {
        id.to_string()
    }
}

pub(crate) fn ident(id: &str) -> RcDoc<'_> {
    RcDoc::text(escaped_ident_name(id))
}

fn pp_ty(ty: &Type) -> RcDoc<'_> {
    use TypeInner::*;
    match ty.as_ref() {
        Null => str("IDL.Null"),
        Bool => str("IDL.Bool"),
        Nat => str("IDL.Nat"),
        Int => str("IDL.Int"),
        Nat8 => str("IDL.Nat8"),
        Nat16 => str("IDL.Nat16"),
        Nat32 => str("IDL.Nat32"),
        Nat64 => str("IDL.Nat64"),
        Int8 => str("IDL.Int8"),
        Int16 => str("IDL.Int16"),
        Int32 => str("IDL.Int32"),
        Int64 => str("IDL.Int64"),
        Float32 => str("IDL.Float32"),
        Float64 => str("IDL.Float64"),
        Text => str("IDL.Text"),
        Reserved => str("IDL.Reserved"),
        Empty => str("IDL.Empty"),
        Var(s) => ident(s.as_str()),
        Principal => str("IDL.Principal"),
        Opt(t) => str("IDL.Opt").append(enclose("(", pp_ty(t), ")")),
        Vec(t) => str("IDL.Vec").append(enclose("(", pp_ty(t), ")")),
        Record(fs) => {
            if is_tuple(ty) {
                let fs = fs.iter().map(|f| pp_ty(&f.ty));
                str("IDL.Tuple").append(sep_enclose(fs, ",", "(", ")"))
            } else {
                str("IDL.Record").append(pp_fields(fs))
            }
        }
        Variant(fs) => str("IDL.Variant").append(pp_fields(fs)),
        Func(func) => str("IDL.Func").append(pp_function(func)),
        Service(serv) => str("IDL.Service").append(pp_service(serv)),
        Class(_, _) => unreachable!(),
        Knot(_) | Unknown | Future => unreachable!(),
    }
}

fn pp_label(id: &SharedLabel) -> RcDoc<'_> {
    match &**id {
        Label::Named(str) => quote_ident(str),
        Label::Id(n) | Label::Unnamed(n) => str("_")
            .append(RcDoc::as_string(n))
            .append("_")
            .append(RcDoc::space()),
    }
}

fn pp_field(field: &Field) -> RcDoc<'_> {
    pp_label(&field.id)
        .append(kwd(":"))
        .append(pp_ty(&field.ty))
}

fn pp_fields(fs: &[Field]) -> RcDoc<'_> {
    sep_enclose_space(fs.iter().map(pp_field), ",", "({", "})")
}

fn pp_function(func: &Function) -> RcDoc<'_> {
    let args = pp_args(&func.args);
    let rets = pp_args(&func.rets);
    let modes = pp_modes(&func.modes);
    sep_enclose([args, rets, modes], ",", "(", ")").nest(INDENT_SPACE)
}

fn pp_args(args: &[Type]) -> RcDoc<'_> {
    pp_types(args.iter())
}

fn pp_types<'a, T>(types: T) -> RcDoc<'a>
where
    T: Iterator<Item = &'a Type>,
{
    sep_enclose(types.map(pp_ty), ",", "[", "]")
}

fn pp_modes(modes: &[candid::types::FuncMode]) -> RcDoc<'_> {
    let ms = modes
        .iter()
        .map(|m| str("'").append(pp_mode(m)).append("'"));
    sep_enclose(ms, ",", "[", "]")
}

/// Check whether `ty` (or any type nested within it) contains a `Var`
/// reference to `name`.
fn references_var(ty: &Type, name: &str) -> bool {
    match ty.as_ref() {
        TypeInner::Var(v) => v.as_str() == name,
        TypeInner::Opt(inner) | TypeInner::Vec(inner) => references_var(inner, name),
        TypeInner::Record(fields) | TypeInner::Variant(fields) => {
            fields.iter().any(|f| references_var(&f.ty, name))
        }
        TypeInner::Func(func) => {
            func.args.iter().any(|a| references_var(a, name))
                || func.rets.iter().any(|r| references_var(r, name))
        }
        TypeInner::Service(methods) => methods.iter().any(|(_, m)| references_var(m, name)),
        TypeInner::Class(args, ty) => {
            args.iter().any(|a| references_var(a, name)) || references_var(ty, name)
        }
        _ => false,
    }
}

/// Find a Service type in `def_list` that has `func_id` as a method field
/// and is in a mutual cycle with it (the Func's args/rets reference the Service).
fn find_service_in_cycle<'a>(
    env: &'a TypeEnv,
    func_id: &str,
    func: &Function,
    def_list: &[&'a str],
    recs: &BTreeSet<String>,
) -> Option<&'a str> {
    for &s_id in def_list {
        if recs.contains(s_id) {
            continue;
        }
        let Ok(s_ty) = env.find_type(s_id) else {
            continue;
        };
        let TypeInner::Service(methods) = s_ty.as_ref() else {
            continue;
        };

        let has_func_field = methods.iter().any(|(_, ty)| references_var(ty, func_id));
        if !has_func_field {
            continue;
        }

        let references_service = func.args.iter().any(|arg| references_var(arg, s_id))
            || func.rets.iter().any(|ret| references_var(ret, s_id));

        if references_service {
            return Some(s_id);
        }
    }
    None
}

/// Run `infer_rec` then `optimize_recs`, returning owned rec names.
fn infer_and_optimize_recs<'a>(env: &'a TypeEnv, def_list: &mut Vec<&'a str>) -> BTreeSet<String> {
    let initial_recs = infer_rec(env, def_list).unwrap();
    let initial_recs: BTreeSet<String> = initial_recs.into_iter().map(|s| s.to_string()).collect();
    optimize_recs(env, def_list, initial_recs)
}

/// Swap Rec placement so that recursive Func types used in Service method
/// fields are emitted as concrete `FuncClass` values instead of `RecClass`.
///
/// `IDL.Service()` requires `Record<string, FuncClass>` for its fields, but
/// `IDL.Func()` args accept any `Type[]`. By making the Service the Rec and
/// the Func a concrete value, both constraints are satisfied.
fn optimize_recs<'a>(
    env: &'a TypeEnv,
    def_list: &mut Vec<&'a str>,
    initial_recs: BTreeSet<String>,
) -> BTreeSet<String> {
    // Collect swaps as owned Strings to avoid borrowing initial_recs.
    // Track claimed services so two Funcs don't both swap with the same one.
    let mut claimed = BTreeSet::new();
    let swaps: Vec<(String, String)> = initial_recs
        .iter()
        .filter_map(|func_id| {
            let ty = env.find_type(func_id.as_str()).ok()?;
            let TypeInner::Func(func) = ty.as_ref() else {
                return None;
            };
            let service_id = find_service_in_cycle(env, func_id, func, def_list, &initial_recs)?;
            if !claimed.insert(service_id) {
                return None;
            }
            Some((func_id.clone(), service_id.to_string()))
        })
        .collect();

    let mut recs = initial_recs;
    for (func_id, service_id) in &swaps {
        recs.remove(func_id);
        recs.insert(service_id.clone());

        // Ensure the func def comes before the service fill in output order.
        if let (Some(fp), Some(sp)) = (
            def_list.iter().position(|&s| s == func_id.as_str()),
            def_list.iter().position(|&s| s == service_id.as_str()),
        ) && fp > sp
        {
            def_list.swap(fp, sp);
        }
    }

    recs
}

fn pp_service(serv: &[(String, Type)]) -> RcDoc<'_> {
    let ms = serv
        .iter()
        .map(|(id, func)| quote_ident(id).append(kwd(":")).append(pp_ty(func)));
    sep_enclose_space(ms, ",", "({", "})")
}

fn pp_defs<'a>(
    env: &'a TypeEnv,
    def_list: &'a [&'a str],
    recs: &'a BTreeSet<&'a str>,
    export: bool,
) -> RcDoc<'a> {
    let export_prefix = if export { str("export ") } else { RcDoc::nil() };

    let recs_doc = lines(recs.iter().map(|id| {
        export_prefix
            .clone()
            .append(kwd("const"))
            .append(ident(id))
            .append(" = IDL.Rec();")
    }));
    let mut defs = lines(def_list.iter().map(|&id| {
        let ty = env.find_type(id).unwrap();
        if recs.contains(id) {
            ident(id)
                .append(".fill")
                .append(enclose("(", pp_ty(ty), ");"))
        } else {
            export_prefix
                .clone()
                .append(kwd("const"))
                .append(ident(id))
                .append(" = ")
                .append(pp_ty(ty))
                .append(";")
        }
    }));
    if !def_list.is_empty() {
        defs = defs.append(RcDoc::hardline())
    }
    recs_doc.append(defs)
}

fn pp_actor<'a>(ty: &'a Type, recs: &'a BTreeSet<&'a str>) -> RcDoc<'a> {
    match ty.as_ref() {
        TypeInner::Service(_) => pp_ty(ty),
        TypeInner::Var(id) => {
            if recs.contains(id.as_str()) {
                ident(id.as_str()).append(".getType()")
            } else {
                ident(id.as_str())
            }
        }
        TypeInner::Class(_, t) => pp_actor(t, recs),
        _ => unreachable!(),
    }
}

fn pp_imports<'a>() -> RcDoc<'a> {
    str("import { IDL } from '@icp-sdk/core/candid';")
        .append(RcDoc::hardline())
        .append(RcDoc::hardline())
}

pub fn compile(env: &TypeEnv, actor: &Option<Type>, root_exports: bool) -> String {
    match actor {
        None => {
            let mut def_list: Vec<_> = env.to_sorted_iter().map(|pair| pair.0.as_str()).collect();
            let initial_recs = infer_rec(env, &def_list).unwrap();
            let initial_recs: BTreeSet<String> =
                initial_recs.into_iter().map(|s| s.to_string()).collect();
            let recs_owned = optimize_recs(env, &mut def_list, initial_recs);
            let recs: BTreeSet<&str> = recs_owned.iter().map(|s| s.as_str()).collect();
            let doc = pp_defs(env, &def_list, &recs, root_exports);

            pp_imports().append(doc).pretty(LINE_WIDTH).to_string()
        }
        Some(actor) => {
            let mut def_list = chase_actor(env, actor).unwrap();
            let initial_recs = infer_rec(env, &def_list).unwrap();
            let initial_recs: BTreeSet<String> =
                initial_recs.into_iter().map(|s| s.to_string()).collect();
            let recs_owned = optimize_recs(env, &mut def_list, initial_recs);
            let recs: BTreeSet<&str> = recs_owned.iter().map(|s| s.as_str()).collect();
            let types = if let TypeInner::Class(args, _) = actor.as_ref() {
                args.clone()
            } else {
                Vec::new()
            };
            let init_types = types.as_slice();

            let actor = pp_actor(actor, &recs);

            let idl_factory_return = kwd("return").append(actor.clone()).append(";");
            let idl_factory_body = pp_defs(env, &def_list, &recs, false).append(idl_factory_return);
            let idl_factory_doc = str("export const idlFactory = ({ IDL }) => ")
                .append(enclose_space("{", idl_factory_body, "};"));

            let init_defs = chase_types(env, init_types).unwrap();
            let init_recs = infer_rec(env, &init_defs).unwrap();
            let init_defs_doc = pp_defs(env, &init_defs, &init_recs, false);
            let init_doc = kwd("return")
                .append(pp_types(init_types.iter()))
                .append(";");
            let init_doc = init_defs_doc.append(init_doc);
            let init_doc =
                str("export const init = ({ IDL }) => ").append(enclose_space("{", init_doc, "};"));
            let init_doc = init_doc.pretty(LINE_WIDTH).to_string();

            let mut result = pp_imports();

            if root_exports {
                let defs = pp_defs(env, &def_list, &recs, true);
                let idl_service = str("export const idlService = ").append(actor).append(";");
                let idl_init_args = str("export const idlInitArgs = ")
                    .append(pp_types(init_types.iter()))
                    .append(";");

                result = result
                    .append(defs)
                    .append(idl_service)
                    .append(RcDoc::hardline())
                    .append(RcDoc::hardline())
                    .append(idl_init_args)
                    .append(RcDoc::hardline())
                    .append(RcDoc::hardline());
            }

            result = result
                .append(idl_factory_doc)
                .append(RcDoc::hardline())
                .append(RcDoc::hardline())
                .append(init_doc);

            result.pretty(LINE_WIDTH).to_string()
        }
    }
}

/// Compiles a merged TypeScript declarations file (`.did.ts`) that combines
/// the TypeScript type definitions with the JavaScript IDL runtime code.
pub fn compile_typescript(
    env: &TypeEnv,
    actor: &Option<Type>,
    prog: &IDLMergedProg,
    root_exports: bool,
) -> String {
    use super::typescript;

    // Render the TypeScript prefix (type imports + type definitions + actor interface)
    // in its own scope to avoid lifetime issues with RcDoc borrowing.
    let ts_prefix = {
        let syntax_actor = prog.resolve_actor().ok().flatten();
        let ts_def_list: Vec<_> = env.to_sorted_iter().map(|pair| pair.0.as_str()).collect();
        let ts_defs = typescript::pp_defs(env, &ts_def_list, prog);

        let ts_actor = match actor {
            None => RcDoc::nil(),
            Some(actor) => {
                let docs = syntax_actor
                    .as_ref()
                    .map(|s| typescript::pp_docs(s.docs.as_ref()))
                    .unwrap_or(RcDoc::nil());
                docs.append(typescript::pp_actor(
                    env,
                    actor,
                    syntax_actor.as_ref().map(|s| &s.typ),
                ))
            }
        };

        typescript::pp_type_imports()
            .append(pp_imports())
            .append(ts_defs)
            .append(ts_actor)
            .pretty(LINE_WIDTH)
            .to_string()
    };

    // Render the JavaScript runtime code with type annotations.
    let js_code = match actor {
        None => {
            let mut def_list: Vec<_> = env.to_sorted_iter().map(|pair| pair.0.as_str()).collect();
            let recs_owned = infer_and_optimize_recs(env, &mut def_list);
            let recs: BTreeSet<&str> = recs_owned.iter().map(|s| s.as_str()).collect();
            let doc = pp_defs(env, &def_list, &recs, root_exports);

            doc.pretty(LINE_WIDTH).to_string()
        }
        Some(actor) => {
            let mut def_list = chase_actor(env, actor).unwrap();
            let recs_owned = infer_and_optimize_recs(env, &mut def_list);
            let recs: BTreeSet<&str> = recs_owned.iter().map(|s| s.as_str()).collect();
            let types = if let TypeInner::Class(args, _) = actor.as_ref() {
                args.clone()
            } else {
                Vec::new()
            };
            let init_types = types.as_slice();

            let actor_expr = pp_actor(actor, &recs);

            let idl_factory_return = kwd("return").append(actor_expr.clone()).append(";");
            let idl_factory_body = pp_defs(env, &def_list, &recs, false).append(idl_factory_return);
            let idl_factory_doc =
                str("export const idlFactory: IDL.InterfaceFactory = ({ IDL }) => ")
                    .append(enclose_space("{", idl_factory_body, "};"));

            let mut init_defs = chase_types(env, init_types).unwrap();
            let init_recs_owned = infer_and_optimize_recs(env, &mut init_defs);
            let init_recs: BTreeSet<&str> = init_recs_owned.iter().map(|s| s.as_str()).collect();
            let init_defs_doc = pp_defs(env, &init_defs, &init_recs, false);
            let init_doc = kwd("return")
                .append(pp_types(init_types.iter()))
                .append(";");
            let init_doc = init_defs_doc.append(init_doc);
            let init_doc =
                str("export const init: (args: { IDL: typeof IDL }) => IDL.Type[] = ({ IDL }) => ")
                    .append(enclose_space("{", init_doc, "};"));
            let init_doc = init_doc.pretty(LINE_WIDTH).to_string();

            let mut result = RcDoc::<()>::nil();

            if root_exports {
                let defs = pp_defs(env, &def_list, &recs, true);
                let idl_service = str("export const idlService: IDL.ServiceClass = ")
                    .append(actor_expr)
                    .append(";");
                let idl_init_args = str("export const idlInitArgs: IDL.Type[] = ")
                    .append(pp_types(init_types.iter()))
                    .append(";");

                result = result
                    .append(defs)
                    .append(idl_service)
                    .append(RcDoc::hardline())
                    .append(RcDoc::hardline())
                    .append(idl_init_args)
                    .append(RcDoc::hardline())
                    .append(RcDoc::hardline());
            }

            result = result
                .append(idl_factory_doc)
                .append(RcDoc::hardline())
                .append(RcDoc::hardline())
                .append(init_doc);

            result.pretty(LINE_WIDTH).to_string()
        }
    };

    format!("{}\n{}\n", ts_prefix, js_code)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Every candid type constructor, each leaf also as the element of a `vec` and an `opt`.
    pub(crate) fn every_constructor() -> Vec<Type> {
        use TypeInner::*;
        let leaves: std::vec::Vec<Type> = [
            Null, Bool, Nat, Int, Nat8, Nat16, Nat32, Nat64, Int8, Int16, Int32, Int64, Float32,
            Float64, Text, Reserved, Empty, Principal,
        ]
        .into_iter()
        .map(Type::from)
        .collect();
        let mut all = leaves.clone();
        all.extend(leaves.iter().map(|t| Type::from(Vec(t.clone()))));
        all.extend(leaves.iter().map(|t| Type::from(Opt(t.clone()))));
        all.push(Type::from(Record(vec![])));
        all.push(Type::from(Variant(vec![])));
        all.push(Type::from(Func(Function {
            modes: vec![],
            args: vec![],
            rets: vec![],
        })));
        all.push(Type::from(Service(vec![])));
        all
    }

    /// The identifiers in a rendered type that a candid type name could collide with: every
    /// bare identifier, i.e. one not reached through a `.`.
    pub(crate) fn bare_identifiers(rendered: &str) -> Vec<String> {
        let mut out = Vec::new();
        let chars: Vec<char> = rendered.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let preceded_by_dot = start > 0 && chars[start - 1] == '.';
                if !preceded_by_dot {
                    out.push(chars[start..i].iter().collect());
                }
            } else {
                i += 1;
            }
        }
        out
    }

    #[test]
    fn javascript_printer_references_only_names_a_candid_type_cannot_take() {
        for ty in every_constructor() {
            let rendered = pp_ty(&ty).pretty(80).to_string();
            for id in bare_identifiers(&rendered) {
                assert_ne!(
                    escaped_ident_name(&id),
                    id,
                    "`{id}` in `{rendered}` is emitted bare but a candid type of that name is \
                     not escaped"
                );
            }
        }
    }
}
