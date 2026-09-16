//! Checks on the candid interface itself, ahead of every generator.

use candid::types::{Label, Type, TypeEnv, TypeInner};

/// A candid name of `__proto__` cannot be carried by any generated file.
///
/// JavaScript treats it as the object's prototype wherever it would appear. `{ __proto__: v }`
/// sets the prototype instead of creating a property, and so does the quoted form, so
/// `IDL.Record({ '__proto__': … })` and `IDL.Service({ '__proto__': … })` in the declarations
/// lose the member from the schema; an enum member of that name lowers to a bracket
/// assignment that hits the same setter; a method overrides an accessor every object
/// inherits. A computed key would rescue a record field alone, but the declared type would
/// still read `__proto__?: T`, so a caller writing the obvious spelling loses the value with
/// no error.
pub(crate) fn check_candid_names(env: &TypeEnv, actor: &Option<Type>) -> Result<(), String> {
    match env.0.values().chain(actor.iter()).any(names_proto) {
        true => Err(
            "the candid name `__proto__` cannot be carried: JavaScript treats it as the \
                     object's prototype wherever it would appear, so the field, tag or method \
                     would be missing at runtime in the declarations and the actor files alike. \
                     Rename it in the .did file."
                .to_string(),
        ),
        false => Ok(()),
    }
}

fn names_proto(ty: &Type) -> bool {
    use TypeInner::*;
    let is_proto = |label: &Label| matches!(label, Label::Named(name) if name == "__proto__");
    match ty.as_ref() {
        Record(fields) | Variant(fields) => fields
            .iter()
            .any(|field| is_proto(&field.id) || names_proto(&field.ty)),
        Service(methods) => methods
            .iter()
            .any(|(name, ty)| name == "__proto__" || names_proto(ty)),
        Func(function) => function
            .args
            .iter()
            .chain(function.rets.iter())
            .any(names_proto),
        Opt(inner) | Vec(inner) => names_proto(inner),
        Class(args, ty) => args.iter().any(names_proto) || names_proto(ty),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::types::Field;

    fn record(field: &str) -> Type {
        TypeInner::Record(vec![Field {
            id: Label::Named(field.to_string()).into(),
            ty: TypeInner::Nat.into(),
        }])
        .into()
    }

    #[test]
    fn proto_field_is_reported_without_an_actor() {
        let mut env = TypeEnv::new();
        env.0.insert("R".to_string(), record("__proto__"));
        assert!(check_candid_names(&env, &None).is_err());
    }

    #[test]
    fn proto_method_of_the_actor_is_reported() {
        let actor: Type = TypeInner::Service(vec![(
            "__proto__".to_string(),
            TypeInner::Func(candid::types::Function {
                modes: vec![],
                args: vec![],
                rets: vec![],
            })
            .into(),
        )])
        .into();
        assert!(check_candid_names(&TypeEnv::new(), &Some(actor)).is_err());
    }

    #[test]
    fn ordinary_names_pass() {
        let mut env = TypeEnv::new();
        env.0.insert("R".to_string(), record("prototype"));
        assert_eq!(check_candid_names(&env, &None), Ok(()));
    }
}
