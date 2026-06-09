//! Raw-HTML allowlist sanitiser for alt-markdown.
//!
//! Built on ammonia (an html5ever-based sanitiser), which is the pairing the
//! prior-art research recommends for a comrak pipeline: render, then sanitise.
//! It keeps a safe subset of HTML and strips scripts, event handlers (`on*`),
//! and dangerous URL schemes (`javascript:` and friends). This is the parse-time
//! defence in the Rust core; the JS runtime adds DOMPurify as defence in depth.

pub mod error;

pub use error::SanitizeError;

/// Sanitise an HTML fragment to a safe subset, removing scripts, event handlers,
/// and dangerous URL schemes.
#[must_use]
pub fn sanitize(html: &str) -> String {
    // The underlying parser strips a byte-order mark only when it leads the
    // input, so a BOM that follows other text survives the first clean but is
    // removed by a second, breaking idempotency. Drop NULs and BOMs up front:
    // neither is meaningful in HTML, and a stray NUL is a classic filter-bypass
    // trick. With them gone, cleaning is stable on its own output.
    let normalised: String = html
        .chars()
        .filter(|&c| c != '\u{0}' && c != '\u{feff}')
        .collect();
    ammonia::clean(&normalised)
}

#[cfg(test)]
mod tests {
    use super::sanitize;
    use proptest::prelude::*;

    #[test]
    fn strips_script_tags() {
        let out = sanitize("<p>ok</p><script>alert(1)</script>");
        assert!(!out.contains("<script"), "script survived: {out}");
        assert!(out.contains("ok"), "safe content dropped: {out}");
    }

    #[test]
    fn strips_event_handlers() {
        let out = sanitize(r#"<img src="x" onerror="alert(1)">"#);
        assert!(
            !out.to_lowercase().contains("onerror"),
            "handler survived: {out}"
        );
    }

    #[test]
    fn strips_javascript_urls() {
        let out = sanitize(r#"<a href="javascript:alert(1)">x</a>"#);
        assert!(!out.contains("javascript:"), "js url survived: {out}");
    }

    #[test]
    fn strips_svg_onload_payload() {
        let out = sanitize(r#"<svg><script>alert(1)</script></svg><img src=x onerror=alert(2)>"#);
        assert!(!out.contains("<script"), "script survived: {out}");
        assert!(
            !out.to_lowercase().contains("onerror"),
            "handler survived: {out}"
        );
    }

    #[test]
    fn is_idempotent_on_bom_and_nul() {
        // Regression: a NUL followed by a BOM left a stray BOM that a second
        // clean removed, so the sanitiser was not idempotent on this input.
        let once = sanitize("\u{0}\u{feff}");
        assert_eq!(sanitize(&once), once, "not idempotent: {once:?}");
        assert_eq!(once, "", "NUL and BOM should both be removed");
    }

    #[test]
    fn keeps_safe_formatting() {
        let out = sanitize("<p><strong>bold</strong> and <em>italic</em></p>");
        assert!(out.contains("<strong>bold</strong>"), "lost strong: {out}");
        assert!(out.contains("<em>italic</em>"), "lost em: {out}");
    }

    proptest! {
        // For arbitrary input the sanitiser never emits an executable tag, and it
        // is idempotent. (Handler/URL substrings can appear as harmless text, so
        // those are asserted in the targeted corpus above, not here.)
        #[test]
        fn never_emits_dangerous_tags_and_is_idempotent(input in ".*") {
            let out = sanitize(&input);
            prop_assert!(!out.contains("<script"));
            prop_assert!(!out.contains("<iframe"));
            prop_assert!(!out.contains("<object"));
            prop_assert!(!out.contains("<embed"));
            prop_assert_eq!(sanitize(&out), out.clone());
        }
    }
}
