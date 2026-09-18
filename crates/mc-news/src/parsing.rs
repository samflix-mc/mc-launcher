//! From markdown to the typed tree.
//!
//! ## A subset, deliberately
//!
//! Paragraphs, headings, bullet lists, separators; bold, italic, code, links.
//! Nothing else — no tables, no raw HTML, no images in the body, no
//! multi-line code blocks.
//!
//! This isn't laziness: every construct we recognize is one more form the
//! front has to know how to render, and one more chance for remote content to
//! produce something unexpected. The rule is that whatever we don't recognize
//! becomes TEXT, never markup — a `<script>` written into a post therefore
//! comes back out as the eleven characters it is.
//!
//! ## Why not a library
//!
//! Full markdown parsers produce HTML. Those that produce a tree pull in a
//! large dependency for a subset we don't use, and whose every future
//! extension would widen the surface without our deciding to. Two hundred
//! lines here can be read in full.

use crate::tree::{Block, Inline};

/// Parses a post body.
pub fn parse(markdown: &str, feed_host: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut items: Vec<Vec<Inline>> = Vec::new();

    // Closes whatever was open before starting something else.
    macro_rules! close {
        ($blocks:expr, $paragraph:expr, $items:expr) => {
            if !$paragraph.is_empty() {
                $blocks.push(Block::Paragraph {
                    content: inlines(&$paragraph.join(" "), feed_host),
                });
                $paragraph.clear();
            }
            if !$items.is_empty() {
                $blocks.push(Block::List {
                    items: std::mem::take(&mut $items),
                });
            }
        };
    }

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            close!(blocks, paragraph, items);
            continue;
        }

        // A separator: three dashes or more, and nothing else.
        if trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-') {
            close!(blocks, paragraph, items);
            blocks.push(Block::Separator);
            continue;
        }

        if let Some((level, text)) = heading(trimmed) {
            close!(blocks, paragraph, items);
            blocks.push(Block::Heading {
                level,
                content: inlines(text, feed_host),
            });
            continue;
        }

        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            // A bullet closes the paragraph but NOT the list: two bullets in
            // a row belong to the same one.
            if !paragraph.is_empty() {
                blocks.push(Block::Paragraph {
                    content: inlines(&paragraph.join(" "), feed_host),
                });
                paragraph.clear();
            }
            items.push(inlines(item.trim(), feed_host));
            continue;
        }

        // An ordinary line after a list closes it.
        if !items.is_empty() {
            blocks.push(Block::List {
                items: std::mem::take(&mut items),
            });
        }
        paragraph.push(trimmed.to_string());
    }

    close!(blocks, paragraph, items);
    blocks
}

/// A heading, and its bounded level.
///
/// The level is brought back into 2..=4: the post's title occupies level 1 of
/// the page, and a `#` in the body must not compete with it — neither in the
/// render, nor for a screen reader, which uses the hierarchy to navigate.
fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    let level = (hashes as u8 + 1).clamp(2, 4);
    Some((level, rest.trim()))
}

/// Parses a line's content.
fn inlines(text: &str, feed_host: &str) -> Vec<Inline> {
    let mut output = Vec::new();
    let mut buffer = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    /// Flushes the buffer into the output, if there's anything in it.
    macro_rules! flush {
        ($output:expr, $buffer:expr) => {
            if !$buffer.is_empty() {
                $output.push(Inline::Text {
                    text: std::mem::take(&mut $buffer),
                });
            }
        };
    }

    while i < chars.len() {
        // A link: [text](url)
        if chars[i] == '['
            && let Some((label, href, skip)) = link(&chars[i..])
        {
            {
                match crate::links::acceptable(&href, feed_host) {
                    Some(safe) => {
                        flush!(output, buffer);
                        output.push(Inline::Link {
                            text: label,
                            href: safe,
                        });
                    }
                    // A refused URL doesn't make the text disappear: the
                    // label becomes ordinary text. Erasing the sentence
                    // because its link is unwelcome would be worse than
                    // keeping it without the link.
                    None => {
                        tracing::warn!(href, "link discarded from a post");
                        buffer.push_str(&label);
                    }
                }
                // `.max(1)`, for the SAME reason as further down: the loop
                // must advance on every turn, and a zero `skip` would spin it
                // forever on the UI thread. `link` never returns zero today —
                // it takes at least `[]()` — but that's a property of `link`,
                // not of this loop, and a loop must not depend for its
                // termination on a function that can be changed elsewhere.
                i += skip.max(1);
                continue;
            }
        }

        // The formatting delimiters, longest to shortest. Whatever isn't one
        // of them is text, character by character: that's the rule that
        // makes a `<script>` written into a post come back out as the eleven
        // characters it is.
        //
        // `advance.max(1)`: the loop MUST advance on every turn.
        //
        // This isn't a theoretical precaution. A post's body comes from a
        // host, it's parsed on the UI thread, and a loop that doesn't advance
        // would freeze the entire window — with no message, no crash, and no
        // way out but killing the process. A parser that can't guarantee its
        // progress has no business being on this path.
        //
        // No real input produces zero today; that's exactly why we write
        // this, rather than discovering the opposite from a player.
        match consume(&chars, i, &mut output, &mut buffer) {
            Some(advance) => i += advance.max(1),
            None => {
                buffer.push(chars[i]);
                i += 1;
            }
        }
    }

    flush!(output, buffer);
    output
}

type Factory = fn(String) -> Inline;

/// The recognized delimiters, LONGEST TO SHORTEST.
///
/// The order isn't decorative: `**bold**` starts with `*`, and testing
/// italic first would produce an empty italic followed by the word then
/// another empty italic.
const DELIMITERS: [(&str, Factory); 3] = [
    ("**", |text| Inline::Bold { text }),
    ("`", |text| Inline::Code { text }),
    ("*", |text| Inline::Italic { text }),
];

/// Attempts to consume a delimiter at position `i`.
fn consume(
    chars: &[char],
    i: usize,
    output: &mut Vec<Inline>,
    buffer: &mut String,
) -> Option<usize> {
    for (marker, factory) in DELIMITERS {
        let marker_chars: Vec<char> = marker.chars().collect();
        if !chars[i..].starts_with(&marker_chars[..]) {
            continue;
        }
        let (inner, skip) = until(&chars[i + marker_chars.len()..], marker)?;
        // An empty delimiter — `****` — isn't formatting: we let it become
        // text rather than producing an empty node that the front would have
        // to know not to render.
        if inner.is_empty() {
            continue;
        }
        if !buffer.is_empty() {
            output.push(Inline::Text {
                text: std::mem::take(buffer),
            });
        }
        output.push(factory(inner));
        return Some(marker_chars.len() + skip);
    }
    None
}

/// The text up to the next occurrence of `marker`, and how far to advance.
///
/// Returns `None` if the marker is never closed: a lone `*` in the middle of
/// a sentence is an asterisk, not an italic left open until the end of the
/// post.
fn until(rest: &[char], marker: &str) -> Option<(String, usize)> {
    let marker_chars: Vec<char> = marker.chars().collect();
    let mut i = 0;
    while i + marker_chars.len() <= rest.len() {
        if rest[i..].starts_with(&marker_chars[..]) {
            return Some((rest[..i].iter().collect(), i + marker_chars.len()));
        }
        i += 1;
    }
    None
}

/// A complete `[label](url)`, or nothing.
fn link(rest: &[char]) -> Option<(String, String, usize)> {
    let close = rest.iter().position(|&c| c == ']')?;
    if rest.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = rest[close + 2..].iter().position(|&c| c == ')')? + close + 2;
    let label: String = rest[1..close].iter().collect();
    let href: String = rest[close + 2..end].iter().collect();
    Some((label, href, end + 1))
}

#[cfg(test)]
#[path = "parsing.test.rs"]
mod tests;
