//! Machine-composed context that reaches a history file through the same `user` channel a person
//! types into. Shared by every adapter whose source does this: Codex injects `<environment_context>`
//! and AGENTS.md dumps; Claude Code writes a `<command-name>…</command-name>` block for each slash
//! command beside the typed `/command` line (measured 2026-09-02 in this repository's own transcript).

const AGENTS_PREFIX: &str = "# AGENTS.md instructions for";

/// True when the whole message is machine-composed context rather than typed words.
///
/// The harness sends context to the model through the same `user` channel a person types into:
/// `<environment_context>`, `<recommended_plugins>`, `<codex_delegation>`, an AGENTS.md dump, and
/// often several of them in one message. Measured on this history, 3,889 of 9,994 Codex user records
/// were injections of this shape. They were never shown as a message, so they do not belong in a
/// visible-conversation index.
///
/// The test is structural rather than a list of names: the message is nothing but complete elements
/// and whitespace, with no prose of its own outside them.
pub(crate) fn is_machine_context(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    if text.starts_with(AGENTS_PREFIX) {
        return true;
    }
    let mut rest = text;
    let mut consumed_one = false;
    while !rest.is_empty() {
        let Some(after) = skip_context_block(rest) else { return false };
        rest = after.trim_start();
        consumed_one = true;
    }
    consumed_one
}

/// The text after one complete context block at the start of `text`: an element with its closing
/// tag, or an AGENTS.md dump, which runs to its `</INSTRUCTIONS>` or to the end. `None` when the
/// text does not open with a block.
fn skip_context_block(text: &str) -> Option<&str> {
    if text.starts_with(AGENTS_PREFIX) {
        const INSTRUCTIONS_END: &str = "</INSTRUCTIONS>";
        return Some(match text.find(INSTRUCTIONS_END) {
            Some(end) => &text[end + INSTRUCTIONS_END.len()..],
            None => "",
        });
    }
    let name = opening_tag_name(text)?;
    let closing = format!("</{name}>");
    let end = text.find(&closing)?;
    Some(&text[end + closing.len()..])
}

/// Name of the element a text opens with, when it opens with one.
fn opening_tag_name(text: &str) -> Option<String> {
    let rest = text.strip_prefix('<')?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_' || *character == '-')
        .collect();
    if name.is_empty() || !rest[name.len()..].starts_with(|c: char| c.is_whitespace() || c == '>') {
        return None;
    }
    Some(name)
}

/// Remove complete context elements that sit before the person's own words.
///
/// A prompt can arrive as an injected element followed by the typed message, for example an
/// `<image …></image>` reference and then the question about it. The element is machine context; the
/// words after it are the visible message.
pub(crate) fn strip_leading_context(text: &str) -> String {
    let mut rest = text.trim_start();
    loop {
        let Some(after) = skip_context_block(rest) else { break };
        let candidate = after.trim_start();
        if candidate.is_empty() {
            break;
        }
        rest = candidate;
    }
    rest.to_string()
}

/// Remove a context element appended after a typed prompt.
///
/// A prompt can arrive as the person's own words followed by an injected block on its own line, for
/// example `pitch me with all templates in an html\n<loom_context …>…</loom_context>`. The words are
/// visible conversation; the block is not.
pub(crate) fn strip_trailing_context(text: &str) -> String {
    let trimmed = text.trim_end();
    let mut cut = None;
    let mut search_from = 0usize;
    while let Some(position) = trimmed[search_from..].find("\n<") {
        let start = search_from + position + 1;
        if let Some(name) = opening_tag_name(&trimmed[start..]) {
            if trimmed.ends_with(&format!("</{name}>")) {
                cut = Some(start);
                break;
            }
        }
        search_from = start;
    }
    match cut {
        Some(start) => trimmed[..start].trim_end().to_string(),
        None => text.to_string(),
    }
}
