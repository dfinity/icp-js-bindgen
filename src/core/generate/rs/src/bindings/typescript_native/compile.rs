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

/// The names the wrapper imports or declares for itself, beside the candid types. Only
/// capitalized ones are listed, since the class name is capitalized.
const PREAMBLE_NAMES: [&str; 11] = [
    // imported from @icp-sdk/core
    "Actor",
    "HttpAgent",
    "HttpAgentOptions",
    "ActorConfig",
    "Agent",
    "ActorSubclass",
    "Principal",
    // declared by the preamble
    "Some",
    "None",
    "Option",
    "CreateActorOptions",
];

/// Describes a collision between the actor class and another declaration in the wrapper.
///
/// The class is named after the `.did` file. An interface of that name merges with the class
/// without an error, a type alias replaces the class's type, and an enum stops the module from
/// loading. Each candid type is declared under its own name, except a service type, which is
/// declared as `<name>Interface`.
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
            if matches!(ty.as_ref(), TypeInner::Service(_)) {
                (get_typescript_ident(&format!("{id}Interface"), true) == class)
                    .then(|| format!("the interface generated for the candid service type `{id}`"))
            } else {
                (get_typescript_ident(id, true) == class).then(|| format!("the candid type `{id}`"))
            }
        })
        .or_else(|| {
            PREAMBLE_NAMES
                .contains(&class.as_str())
                .then(|| format!("`{class}`, which the generated wrapper declares"))
        })?;

    Some(format!(
        "The actor class `{class}`, named after {service_name}.did, has the same name as \
         {collision}. Rename the .did file, or disable the actor output to generate only the \
         declarations."
    ))
}
