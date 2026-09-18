use super::parse;
use crate::tree::{Block, Inline};

const FEED: &str = "https://mc-launcher.ggy.info/pack/news.json";

fn text(t: &str) -> Inline {
    Inline::Text { text: t.into() }
}

fn parsed(markdown: &str) -> Vec<Block> {
    parse(markdown, FEED)
}

// --- Blocks ------------------------------------------------------------

#[test]
fn a_paragraph_is_a_paragraph() {
    assert_eq!(
        parsed("Hello."),
        vec![Block::Paragraph {
            content: vec![text("Hello.")]
        }]
    );
}

/// Two lines in a row belong to the SAME paragraph: that's the markdown
/// rule, and without it a post written with soft line breaks would render as
/// a series of one-line paragraphs.
#[test]
fn two_lines_in_a_row_make_one_paragraph() {
    assert_eq!(
        parsed("One line\nand the rest"),
        vec![Block::Paragraph {
            content: vec![text("One line and the rest")]
        }]
    );
}

#[test]
fn a_blank_line_separates_two_paragraphs() {
    assert_eq!(parsed("One.\n\nTwo.").len(), 2);
}

/// The level is bounded to 2..=4: the post's title occupies level 1 of the
/// page. A `#` in the body must not compete with it — neither in the render,
/// nor for a screen reader, which uses the hierarchy to navigate.
#[test]
fn headings_are_bounded_to_two_through_four() {
    for (markdown, expected) in [
        ("# One", 2u8),
        ("## Two", 3),
        ("### Three", 4),
        ("#### Four", 4),
        ("###### Six", 4),
    ] {
        match &parsed(markdown)[0] {
            Block::Heading { level, .. } => assert_eq!(*level, expected, "{markdown}"),
            other => panic!("{markdown} is not a heading: {other:?}"),
        }
    }
}

/// Beyond six hashes, it's no longer a heading — it's text.
///
/// The upper bound isn't decorative: CommonMark stops at six, and a line of
/// seven hashes is almost always a decorative separator typed by hand.
/// Rendering it as a level-4 heading would place, in the document's
/// hierarchy — the one a screen reader follows — a level nobody meant to put
/// there.
///
/// The test covers SEVEN and not six: it's the first case where the bound
/// decides, and the only one that distinguishes `hashes == 0 || hashes > 6`
/// from the same line written with `&&`, which would never be true and would
/// let everything through.
#[test]
fn beyond_six_hashes_it_is_no_longer_a_heading() {
    for markdown in ["####### Seven", "######## Eight"] {
        assert_eq!(
            parsed(markdown),
            vec![Block::Paragraph {
                content: vec![text(markdown)]
            }],
            "{markdown}"
        );
    }
}

/// A hash WITHOUT a space isn't a heading: it's a hashtag, and posts contain
/// them.
#[test]
fn a_hash_without_a_space_is_not_a_heading() {
    assert_eq!(
        parsed("#season3 arrives"),
        vec![Block::Paragraph {
            content: vec![text("#season3 arrives")]
        }]
    );
}

#[test]
fn bullets_make_a_single_list() {
    assert_eq!(
        parsed("- one\n- two\n- three"),
        vec![Block::List {
            items: vec![vec![text("one")], vec![text("two")], vec![text("three")]]
        }]
    );
    // Asterisks too: both forms exist in the wild.
    assert_eq!(parsed("* one\n* two").len(), 1);
}

/// An ordinary line after a list CLOSES it. Without that, the rest of the
/// post would become one big bullet.
#[test]
fn an_ordinary_line_closes_the_list() {
    let blocks = parsed("- one\n- two\nA sentence.");
    assert_eq!(blocks.len(), 2);
    assert!(matches!(blocks[0], Block::List { .. }));
    assert!(matches!(blocks[1], Block::Paragraph { .. }));
}

#[test]
fn three_dashes_make_a_separator() {
    assert_eq!(parsed("---"), vec![Block::Separator]);
    assert_eq!(parsed("-----"), vec![Block::Separator]);
    // Two dashes don't make one: that's text.
    assert!(matches!(parsed("--")[0], Block::Paragraph { .. }));
}

#[test]
fn an_empty_body_gives_no_blocks() {
    assert!(parsed("").is_empty());
    assert!(parsed("\n\n  \n").is_empty());
}

// --- Inlines -------------------------------------------------------------

#[test]
fn bold_italic_and_code() {
    assert_eq!(
        parsed("a **bold** b *italic* c `code` d"),
        vec![Block::Paragraph {
            content: vec![
                text("a "),
                Inline::Bold {
                    text: "bold".into()
                },
                text(" b "),
                Inline::Italic {
                    text: "italic".into()
                },
                text(" c "),
                Inline::Code {
                    text: "code".into()
                },
                text(" d"),
            ]
        }]
    );
}

/// The ORDER of delimiters isn't decorative: `**bold**` starts with `*`, and
/// testing italic first would produce an empty italic, then the word, then
/// another empty italic.
#[test]
fn bold_wins_over_italic() {
    match &parsed("**important**")[0] {
        Block::Paragraph { content } => {
            assert_eq!(content.len(), 1, "{content:?}");
            assert!(matches!(content[0], Inline::Bold { .. }), "{content:?}");
        }
        other => panic!("{other:?}"),
    }
}

/// A delimiter that's never closed is an ordinary character. A lone `*` in
/// the middle of a sentence — "3 * 4" — must not put the rest of the post in
/// italics.
#[test]
fn a_delimiter_never_closed_is_text() {
    assert_eq!(
        parsed("3 * 4 = 12"),
        vec![Block::Paragraph {
            content: vec![text("3 * 4 = 12")]
        }]
    );
}

/// `****` isn't empty formatting: it's text. Producing an empty node would
/// force the front to know not to render it.
#[test]
fn an_empty_delimiter_is_text() {
    assert_eq!(
        parsed("****"),
        vec![Block::Paragraph {
            content: vec![text("****")]
        }]
    );
}

// --- What this module exists to prevent -----------------------------------

/// THE crate's guarantee: whatever isn't recognized becomes TEXT, never
/// markup. A `<script>` written into a post comes back out as the characters
/// it is, and the front will display it as such.
///
/// This is the Rust half of the condition under which the CSP was loosened.
#[test]
fn no_markup_survives_parsing() {
    let poisonous = "<script>alert(1)</script> and <img onerror=alert(2)>";
    match &parsed(poisonous)[0] {
        Block::Paragraph { content } => {
            assert_eq!(content, &vec![text(poisonous)]);
        }
        other => panic!("{other:?}"),
    }
}

/// An acceptable link becomes a `Link` with its href.
#[test]
fn a_link_becomes_a_link() {
    match &parsed("see [the mod](https://modrinth.com/mod/jei) here")[0] {
        Block::Paragraph { content } => {
            assert_eq!(content[0], text("see "));
            assert_eq!(
                content[1],
                Inline::Link {
                    text: "the mod".into(),
                    href: "https://modrinth.com/mod/jei".into()
                }
            );
            assert_eq!(content[2], text(" here"));
        }
        other => panic!("{other:?}"),
    }
}

/// A refused link doesn't make the SENTENCE disappear: the label becomes
/// ordinary text. Erasing a sentence because its link is unwelcome would be
/// worse than keeping it without the link.
#[test]
fn a_refused_link_keeps_its_label_as_text() {
    match &parsed("click [here](javascript:alert(1)) quick")[0] {
        Block::Paragraph { content } => {
            assert!(
                !content.iter().any(|i| matches!(i, Inline::Link { .. })),
                "a dangerous link survived: {content:?}"
            );
            let whole: String = content
                .iter()
                .map(|i| match i {
                    Inline::Text { text } => text.clone(),
                    other => format!("{other:?}"),
                })
                .collect();
            assert!(whole.contains("here"), "the label disappeared: {whole}");
            assert!(whole.contains("click"), "{whole}");
        }
        other => panic!("{other:?}"),
    }
}

/// A bracket that doesn't open a link stays a bracket.
#[test]
fn a_lone_bracket_is_text() {
    assert_eq!(
        parsed("a [thing] without a link"),
        vec![Block::Paragraph {
            content: vec![text("a [thing] without a link")]
        }]
    );
}
