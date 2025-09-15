//! Ported from https://github.com/dfinity/candid/blob/1ddf879f368f765145223c08bbe2c8c8f4782dcc/rust/candid_parser/src/bindings/javascript.rs

use candid::pretty::candid::pp_mode;
use candid::pretty::utils::*;
use candid::types::{ArgType, Field, Function, Label, SharedLabel, Type, TypeEnv, TypeInner};
use candid_parser::bindings::analysis::{chase_actor, chase_types, infer_rec};
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

static KEYWORDS: [&str; 64] = [
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
pub(crate) fn ident(id: &str) -> RcDoc<'_> {
    if KEYWORDS.contains(&id) {
        str(id).append("_")
    } else {
        str(id)
    }
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
    let rets = pp_rets(&func.rets);
    let modes = pp_modes(&func.modes);
    sep_enclose([args, rets, modes], ",", "(", ")").nest(INDENT_SPACE)
}

fn pp_args(args: &[ArgType]) -> RcDoc<'_> {
    let args = args.iter().map(|arg| pp_ty(&arg.typ));
    sep_enclose(args, ",", "[", "]")
}

fn pp_rets(args: &[Type]) -> RcDoc<'_> {
    sep_enclose(args.iter().map(pp_ty), ",", "[", "]")
}

fn pp_modes(modes: &[candid::types::FuncMode]) -> RcDoc<'_> {
    let ms = modes
        .iter()
        .map(|m| str("'").append(pp_mode(m)).append("'"));
    sep_enclose(ms, ",", "[", "]")
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
        let ty = env.find_type(&id.into()).unwrap();
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
                str(id.as_str()).append(".getType()")
            } else {
                str(id.as_str())
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

pub fn compile(env: &TypeEnv, actor: &Option<Type>) -> String {
    match actor {
        None => {
            let def_list: Vec<_> = env.to_sorted_iter().map(|pair| pair.0.as_str()).collect();
            let recs = infer_rec(env, &def_list).unwrap();
            let doc = pp_defs(env, &def_list, &recs, true);
            let result = pp_imports().append(doc).pretty(LINE_WIDTH).to_string();

            result
        }
        Some(actor) => {
            let def_list = chase_actor(env, actor).unwrap();
            let recs = infer_rec(env, &def_list).unwrap();
            let types = if let TypeInner::Class(args, _) = actor.as_ref() {
                args.iter().map(|arg| arg.typ.clone()).collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let init_types = types.as_slice();

            let defs = pp_defs(env, &def_list, &recs, true);
            let actor = pp_actor(actor, &recs);

            let idl_service = str("export const idlService = ")
                .append(actor.clone())
                .append(";");

            let idl_init_args = str("export const idlInitArgs = ")
                .append(pp_rets(init_types))
                .append(";");

            let idl_factory_return = kwd("return").append(actor).append(";");
            let idl_factory_body = pp_defs(env, &def_list, &recs, false).append(idl_factory_return);
            let idl_factory_doc = str("export const idlFactory = ({ IDL }) => ")
                .append(enclose_space("{", idl_factory_body, "};"));

            let init_defs = chase_types(env, init_types).unwrap();
            let init_recs = infer_rec(env, &init_defs).unwrap();
            let init_defs_doc = pp_defs(env, &init_defs, &init_recs, false);
            let init_doc = kwd("return").append(pp_rets(init_types)).append(";");
            let init_doc = init_defs_doc.append(init_doc);
            let init_doc =
                str("export const init = ({ IDL }) => ").append(enclose_space("{", init_doc, "};"));
            let init_doc = init_doc.pretty(LINE_WIDTH).to_string();

            let result = pp_imports()
                .append(defs)
                .append(idl_service)
                .append(RcDoc::hardline())
                .append(RcDoc::hardline())
                .append(idl_init_args)
                .append(RcDoc::hardline())
                .append(RcDoc::hardline())
                .append(idl_factory_doc)
                .append(RcDoc::hardline())
                .append(RcDoc::hardline())
                .append(init_doc);

            result.pretty(LINE_WIDTH).to_string()
        }
    }
}
