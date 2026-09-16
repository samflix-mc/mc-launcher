//! Ce qu'on autorise la résolution à faire.

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Suivre les dépendances annoncées par les API.
    ///
    /// Les désactiver ne casse rien : le rattrapage par lecture des jars
    /// retrouve les mêmes dépendances, simplement un tour plus tard. C'est ce
    /// qui permet de vérifier que ce rattrapage fonctionne — et de s'en
    /// remettre uniquement à ce que le jeu lira, quand une fiche de
    /// publication est fautive.
    pub follow_declared: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            follow_declared: true,
        }
    }
}
