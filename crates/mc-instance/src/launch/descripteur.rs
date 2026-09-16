//! Les `version.json` de Mojang et du chargeur.

mod chaine;

use serde::Deserialize;

use crate::vanilla;

pub(in crate::launch) use chaine::{library_key, resolve_chain};

use crate::vanilla::Library;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct VersionJson {
    pub(super) id: String,
    pub(super) inherits_from: Option<String>,
    pub(super) main_class: Option<String>,
    pub(super) asset_index: Option<AssetIndex>,
    pub(super) assets: Option<String>,
    #[serde(default)]
    pub(super) libraries: Vec<Library>,
    #[serde(default)]
    pub(super) arguments: Arguments,
    /// Format d'avant 2017, encore présent sur les très vieilles versions.
    pub(super) minecraft_arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AssetIndex {
    pub(super) id: String,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct Arguments {
    #[serde(default)]
    pub(super) game: Vec<Argument>,
    #[serde(default)]
    pub(super) jvm: Vec<Argument>,
}

/// Un argument : soit une chaîne, soit une valeur sous condition.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum Argument {
    Simple(String),
    Conditional {
        #[serde(default)]
        rules: Vec<vanilla::Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}

impl ArgumentValue {
    pub(super) fn parts(&self) -> &[String] {
        match self {
            ArgumentValue::One(v) => std::slice::from_ref(v),
            ArgumentValue::Many(v) => v,
        }
    }
}
