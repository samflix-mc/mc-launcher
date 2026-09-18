use super::{Rapport, manquements};

/// Ce que la fenêtre rapporte quand tout va — relevé le 18 septembre 2026 sur
/// un `tauri build --debug`.
fn bon() -> Rapport {
    Rapport {
        theme: "oklch(58% 0.13 133)".into(),
        verre: "blur(14px) saturate(140%)".into(),
        color_mix: "rgb(128, 0, 128)".into(),
        route_montee: true,
        feuilles: 1,
        violations: Vec::new(),
    }
}

#[test]
fn un_build_conforme_ne_manque_de_rien() {
    assert_eq!(manquements(&bon()), Vec::<String>::new());
}

/// **Le point qui justifie tout le module.**
///
/// Un `backdrop-filter` qui ne rend rien veut dire l'une de deux choses, et
/// aucune des deux ne se voit en `ng serve` : ou bien le style de composant
/// est rejeté — c'est le desserrage de `style-src` qui est tombé — ou bien
/// esbuild a cessé de préfixer, et WebKitGTK ne connaît que la forme
/// préfixée. Dans les deux cas la fenêtre s'ouvre, et le verre est un aplat.
#[test]
fn un_verre_absent_est_signale() {
    for valeur in ["", "none"] {
        let rapport = Rapport {
            verre: valeur.into(),
            ..bon()
        };
        let manques = manquements(&rapport);
        assert_eq!(manques.len(), 1, "verre « {valeur} »");
        assert!(manques[0].contains("backdrop-filter"));
    }
}

/// Aucun composant de route monté : le fragment paresseux n'est pas arrivé.
/// C'est le seul point qui éprouve `import()` sous `script-src`, et
/// l'affirmation qu'il fonctionne n'avait jamais été démontrée.
///
/// **L'indicateur a changé une fois, et la raison mérite d'être gardée.** On
/// comptait d'abord les fichiers `.js` par `performance.getEntriesByType`,
/// et la sonde rapportait ZÉRO sur un build parfaitement sain : le protocole
/// d'actifs de Tauri ne renseigne pas la Resource Timing API. Un contrôle qui
/// crie au loup sur un build correct se désactive en deux jours ; celui-ci
/// observe désormais le DOM, qui ne peut pas mentir sur ce point.
#[test]
fn un_fragment_paresseux_manquant_est_signale() {
    let manques = manquements(&Rapport {
        route_montee: false,
        ..bon()
    });
    assert_eq!(manques.len(), 1);
    assert!(manques[0].contains("paresseux"));
}

#[test]
fn un_theme_absent_est_signale() {
    let manques = manquements(&Rapport {
        theme: String::new(),
        ..bon()
    });
    assert_eq!(manques.len(), 1);
    assert!(manques[0].contains("--color-primary"));
}

#[test]
fn une_feuille_de_style_absente_est_signalee() {
    let manques = manquements(&Rapport {
        feuilles: 0,
        ..bon()
    });
    assert_eq!(manques.len(), 1);
    assert!(manques[0].contains("<link>"));
}

/// `color-mix()` non résolu ressort tel qu'il a été écrit, ou vide. Le CSS
/// compilé en porte cent trente occurrences : si le moteur ne le résout pas,
/// c'est tout le thème daisyUI qui tombe.
#[test]
fn un_color_mix_non_resolu_est_signale() {
    for valeur in ["", "color-mix(in oklab, red 50%, blue)"] {
        let manques = manquements(&Rapport {
            color_mix: valeur.into(),
            ..bon()
        });
        assert_eq!(manques.len(), 1, "color-mix « {valeur} »");
        assert!(manques[0].contains("color-mix"));
    }
}

/// Les deux formes qu'un moteur peut rendre sont acceptées : WebKit répond en
/// `rgb(…)`, mais rien n'oblige un moteur à convertir hors de l'espace demandé.
#[test]
fn les_deux_formes_resolues_passent() {
    for valeur in ["rgb(128, 0, 128)", "oklab(0.5 0.1 -0.1)"] {
        assert!(
            manquements(&Rapport {
                color_mix: valeur.into(),
                ..bon()
            })
            .is_empty(),
            "color-mix « {valeur} »"
        );
    }
}

/// Une violation de CSP est reportée TELLE QUELLE : c'est la directive et
/// l'URL bloquée qui disent quoi corriger, et les reformuler ferait perdre la
/// seule information utile.
#[test]
fn chaque_violation_est_reportee() {
    let manques = manquements(&Rapport {
        violations: vec![
            "script-src ← inline".into(),
            "img-src ← https://ailleurs".into(),
        ],
        ..bon()
    });
    assert_eq!(manques.len(), 2);
    assert!(manques[0].contains("script-src"));
    assert!(manques[1].contains("https://ailleurs"));
}

/// Un rapport vide — ce que rend une charge utile illisible — ne doit pas
/// passer pour un succès. C'est le cas qui compte le plus : une sonde qui se
/// tait quand elle échoue est pire que pas de sonde.
#[test]
fn un_rapport_vide_ne_passe_pas_pour_un_succes() {
    let manques = manquements(&Rapport::default());
    assert!(
        manques.len() >= 4,
        "un rapport vide doit tout signaler : {manques:?}"
    );
}
