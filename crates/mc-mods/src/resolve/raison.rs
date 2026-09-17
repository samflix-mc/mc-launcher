//! Pourquoi un mod se retrouve dans le pack, et ce que ça lui donne de poids.

use super::demande::Request;

/// ni s'il peut être retiré quand son parent l'est.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// Nommé dans le manifeste.
    Explicit,
    /// Dépendance déclarée par l'API de la source.
    Declared { by: String },
    /// Dépendance qu'aucune API n'annonçait, lue dans le descripteur d'un jar.
    Implicit { by: String, mod_id: String },
}

impl Reason {
    pub fn describe(&self) -> String {
        match self {
            Reason::Explicit => "demandé par le manifeste".to_string(),
            Reason::Declared { by } => format!("dépendance déclarée de {by}"),
            Reason::Implicit { by, mod_id } => {
                format!("dépendance implicite : {by} exige « {mod_id} »")
            }
        }
    }
}

/// Ce qui départage deux branches qui réclament le même projet.
///
/// Deux critères, et **l'épinglage passe avant l'origine** :
///
/// | | épinglée | ouverte |
/// |---|---|---|
/// | manifeste | 12 | 4 |
/// | dépendance déclarée | 10 | 2 |
/// | exigence lue dans un jar | 8 | 0 |
///
/// Sans cet ordre, le premier arrivé gardait la place — donc le hasard du
/// parcours décidait de la version installée.
///
/// ## Pourquoi l'épinglage l'emporte sur l'origine
///
/// Le manifeste a longtemps primé en toutes circonstances, y compris sans
/// version. C'était un angle mort : **une demande sans version n'exprime aucune
/// préférence de version**. Écrire « sodium » dit « je veux ce mod », pas « je
/// veux sa dernière version quoi qu'il en coûte ».
///
/// Le cas qui l'a montré : un pack demandait « sodium » sans version, Iris
/// déclarait une dépendance vers un build précis de Sodium — ses mixins de
/// compatibilité visent des classes qui changent de nom d'une version à
/// l'autre. L'ancienne règle donnait la dernière version à Sodium, les mixins
/// d'Iris s'appliquaient dans le vide, et Minecraft tombait à la première
/// connexion sur une classe disparue.
///
/// Le manifeste reste souverain **dès qu'il dit quelque chose** : une demande
/// qu'il épingle bat tout le reste. C'est la même règle que cargo ou npm — une
/// contrainte stricte l'emporte sur « n'importe quelle version ».
pub(super) fn autorite(reason: &Reason, request: &Request) -> u8 {
    let origine = match reason {
        Reason::Explicit => 4,
        Reason::Declared { .. } => 2,
        Reason::Implicit { .. } => 0,
    };
    // Huit : plus que l'écart maximal entre deux origines, donc aucune origine
    // ne rattrape un épinglage.
    let epinglee = request.file.is_some() || request.version.is_some();
    origine + if epinglee { 8 } else { 0 }
}

/// Une exigence lue dans un jar vient-elle de perdre définitivement sa place ?
///
/// Vrai quand la demande est implicite, qu'un demandeur au moins aussi
/// autoritaire tient déjà la clé, et qu'il y tient un autre build. Le tour
/// suivant relèverait le même manque, proposerait le même projet et le
/// reperdrait à l'identique : la file ne se viderait jamais et la résolution
/// mourait sur [`MAX_PASSES`], en accusant des dépendances circulaires qui
/// n'existent pas. Le manque est consigné une fois, et l'installation continue.
pub(super) fn impasse_implicite(
    reason: &Reason,
    entrante: u8,
    retenue: u8,
    meme_build: bool,
) -> bool {
    matches!(reason, Reason::Implicit { .. }) && entrante <= retenue && !meme_build
}

#[cfg(test)]
#[path = "raison.test.rs"]
mod tests;
