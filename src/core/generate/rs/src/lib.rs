mod bindings;
mod fs;
mod parser;

use std::path::PathBuf;

use serde::Deserialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::bindings::{javascript, typescript, typescript_native};

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
    match log::set_logger(&wasm_bindgen_console_logger::DEFAULT_LOGGER) {
        Ok(_) => log::info!("Console logger initialized"),
        Err(e) => log::error!("Failed to set console logger: {}", e),
    }
    log::set_max_level(log::LevelFilter::Trace);
}

#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi)]
pub struct GenerateDeclarationsOptions {
    pub root_exports: bool,
}

#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi)]
pub enum DidFile {
    LocalPath(String),
    InlineString(String),
}

#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi)]
pub struct GenerateOptions {
    pub did_file: DidFile,
    pub service_name: String,
    pub declarations: GenerateDeclarationsOptions,
}

#[wasm_bindgen(getter_with_clone)]
pub struct GenerateResult {
    pub declarations_js: String,
    pub declarations_ts: String,
    pub interface_ts: String,
    pub service_ts: String,
}

#[wasm_bindgen]
pub fn generate(options: GenerateOptions) -> Result<GenerateResult, JsError> {
    let (env, actor, prog) = match options.did_file {
        DidFile::LocalPath(did_file_path) => {
            let input_path = PathBuf::from(did_file_path);
            parser::check_file(input_path.as_path()).map_err(JsError::from)?
        }
        DidFile::InlineString(did_file_str) => {
            parser::check_str(&did_file_str).map_err(JsError::from)?
        }
    };

    let declarations_js = javascript::compile(&env, &actor, options.declarations.root_exports);
    let declarations_ts =
        typescript::compile(&env, &actor, &prog, options.declarations.root_exports);

    let interface_ts = typescript_native::compile::compile(
        &env,
        &actor,
        &options.service_name,
        "interface",
        &prog,
    );

    let service_ts =
        typescript_native::compile::compile(&env, &actor, &options.service_name, "wrapper", &prog);

    Ok(GenerateResult {
        declarations_js,
        declarations_ts,
        interface_ts,
        service_ts,
    })
}
