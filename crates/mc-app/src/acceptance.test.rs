use super::{Report, gaps};

/// What the window reports when everything's fine — recorded on September
/// 18, 2026 on a `tauri build --debug`.
fn good() -> Report {
    Report {
        theme: "#e6b54a".into(),
        glass: "blur(24px) saturate(160%)".into(),
        component_style: "applied".into(),
        color_mix: "rgb(128, 0, 128)".into(),
        route_mounted: true,
        sheets: 1,
        violations: Vec::new(),
    }
}

#[test]
fn a_conformant_build_is_missing_nothing() {
    assert_eq!(gaps(&good()), Vec::<String>::new());
}

/// **The point that justifies the whole module.**
///
/// A `backdrop-filter` that renders nothing means one of two things, and
/// neither shows up in `ng serve`: either the component style is rejected —
/// the `style-src` loosening fell through — or esbuild stopped prefixing,
/// and WebKitGTK only knows the prefixed form. Either way the window opens,
/// and the glass is flat.
#[test]
fn a_missing_glass_is_reported() {
    for value in ["", "none"] {
        let report = Report {
            glass: value.into(),
            ..good()
        };
        let gaps = gaps(&report);
        assert_eq!(gaps.len(), 1, "glass \"{value}\"");
        assert!(gaps[0].contains("backdrop-filter"));
    }
}

/// No route component mounted: the lazy fragment didn't arrive. It's the
/// one point that tests `import()` under `script-src`, and the claim that
/// it works had never been demonstrated.
///
/// **The indicator changed once, and the reason is worth keeping.** It used
/// to count `.js` files through `performance.getEntriesByType`, and the
/// probe reported ZERO on a perfectly sound build: Tauri's asset protocol
/// doesn't feed the Resource Timing API. A check that cries wolf on a
/// correct build gets disabled within two days; this one now observes the
/// DOM, which can't lie about this point.
#[test]
fn a_missing_lazy_fragment_is_reported() {
    let gaps = gaps(&Report {
        route_mounted: false,
        ..good()
    });
    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].contains("lazy"));
}

#[test]
fn missing_tokens_are_reported() {
    let gaps = gaps(&Report {
        theme: String::new(),
        ..good()
    });
    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].contains("--gold"));
}

/// **The test for `style-src` being loosened, and it no longer has any
/// other carrier.**
///
/// The marker property is declared in `web/src/app/app.css`, i.e. in a
/// component's `styleUrl`, and nowhere else. If it doesn't arrive, it's
/// because the window rejected component styles: pages then display with
/// none of their layout, and the console says nothing more than a CSP
/// violation nobody reads.
#[test]
fn a_rejected_component_style_is_reported() {
    let gaps = gaps(&Report {
        component_style: String::new(),
        ..good()
    });
    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].contains("style-src"), "{gaps:?}");
}

#[test]
fn a_missing_stylesheet_is_reported() {
    let gaps = gaps(&Report {
        sheets: 0,
        ..good()
    });
    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].contains("<link>"));
}

/// An unresolved `color-mix()` comes back exactly as written, or empty. The
/// design system uses it for the glass's tint, the outlines, the state hues
/// and the scrim: if the engine doesn't resolve it, it's not one color that
/// falls through, it's every surface of the window.
#[test]
fn an_unresolved_color_mix_is_reported() {
    for value in ["", "color-mix(in oklab, red 50%, blue)"] {
        let gaps = gaps(&Report {
            color_mix: value.into(),
            ..good()
        });
        assert_eq!(gaps.len(), 1, "color-mix \"{value}\"");
        assert!(gaps[0].contains("color-mix"));
    }
}

/// Both forms an engine can render are accepted: WebKit answers in
/// `rgb(…)`, but nothing forces an engine to convert outside the requested
/// space.
#[test]
fn both_resolved_forms_pass() {
    for value in ["rgb(128, 0, 128)", "oklab(0.5 0.1 -0.1)"] {
        assert!(
            gaps(&Report {
                color_mix: value.into(),
                ..good()
            })
            .is_empty(),
            "color-mix \"{value}\""
        );
    }
}

/// A CSP violation is reported AS-IS: it's the directive and the blocked
/// URL that say what to fix, and rewording them would lose the only useful
/// information.
#[test]
fn every_violation_is_reported() {
    let gaps = gaps(&Report {
        violations: vec![
            "script-src ← inline".into(),
            "img-src ← https://elsewhere".into(),
        ],
        ..good()
    });
    assert_eq!(gaps.len(), 2);
    assert!(gaps[0].contains("script-src"));
    assert!(gaps[1].contains("https://elsewhere"));
}

/// An empty report — what an unreadable payload returns — must not pass for
/// a success. This is the case that matters most: a probe that stays quiet
/// when it fails is worse than no probe.
#[test]
fn an_empty_report_does_not_pass_for_a_success() {
    let gaps = gaps(&Report::default());
    assert!(
        gaps.len() >= 4,
        "an empty report must report everything: {gaps:?}"
    );
}
