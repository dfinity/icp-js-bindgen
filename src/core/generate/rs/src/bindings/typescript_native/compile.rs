use super::compile_interface::compile_interface;
use super::compile_wrapper::compile_wrapper;
use super::utils::get_typescript_ident;
use candid::types::{Type, TypeEnv, TypeInner};
use candid_parser::syntax::IDLMergedProg;

pub fn compile(
    env: &TypeEnv,
    actor: &Option<Type>,
    service_name: &str,
    target: &str,
    prog: &IDLMergedProg,
) -> String {
    if target == "interface" {
        compile_interface(env, actor, service_name, prog)
    } else if target == "wrapper" {
        compile_wrapper(env, actor, service_name, prog)
    } else {
        panic!("Invalid target: {}", target);
    }
}

/// The interfaces the wrapper declares beside the actor class, other than the candid types.
const PREAMBLE_INTERFACES: [&str; 3] = ["Some", "None", "CreateActorOptions"];

/// Describes a collision between the actor class and an interface the wrapper declares.
///
/// The class is named after the `.did` file. TypeScript merges a class and an interface of
/// one name without an error, so the class would claim the interface's members and the
/// interface the class's. Every other declaration of that name is a TypeScript error already.
pub fn actor_class_collision(
    env: &TypeEnv,
    actor: &Option<Type>,
    service_name: &str,
) -> Option<String> {
    actor.as_ref()?;

    let mut chars = service_name.chars();
    let capitalized: String = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => return None,
    };
    let class = get_typescript_ident(&capitalized, true);

    let collision = env
        .0
        .iter()
        .find_map(|(id, ty)| {
            if get_typescript_ident(id, true) == class {
                return Some(format!("the candid type `{id}`"));
            }
            let is_service = env
                .trace_type(ty)
                .is_ok_and(|t| matches!(t.as_ref(), TypeInner::Service(_)));
            (is_service && get_typescript_ident(&format!("{id}Interface"), true) == class)
                .then(|| format!("the interface generated for the candid service type `{id}`"))
        })
        .or_else(|| {
            PREAMBLE_INTERFACES
                .contains(&class.as_str())
                .then(|| format!("the generated `{class}` interface"))
        })?;

    Some(format!(
        "The actor class `{class}`, named after {service_name}.did, has the same name as \
         {collision}. TypeScript would merge the two. Rename the .did file, or disable the \
         actor output to generate only the declarations."
    ))
}
