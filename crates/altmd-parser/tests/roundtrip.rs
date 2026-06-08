//! Round-trip tests for the serializer.
//!
//! The contract is a *normalising* round-trip: serialising a parsed document and
//! re-parsing it yields an AST equal to the original. Byte-identical output is
//! explicitly not promised in v0.1. The property therefore asserts AST equality
//! after the round-trip, not source equality.

use altmd_ast::{Parser as _, Serializer as _};
use altmd_parser::{CommonMarkParser, MarkdownSerializer};
use proptest::collection::vec;
use proptest::prelude::*;

/// Fixpoint round-trip: the normal form is stable. One normalisation pass may
/// canonicalise the AST (for example two adjacent same-type lists, which markdown
/// represents as one, or an unreferenced footnote definition, which comrak drops),
/// but every pass thereafter is a no-op. This is the general contract a
/// normalising serializer can guarantee for arbitrary input.
fn round_trip_is_stable(source: &str) -> Result<(), TestCaseError> {
    let parser = CommonMarkParser::new();
    let serializer = MarkdownSerializer::new();
    let Ok(first) = parser.parse(source) else {
        return Ok(());
    };
    // The normal form: parse, serialise, parse again.
    let normal = parser
        .parse(&serializer.to_source(&first))
        .map_err(|e| TestCaseError::fail(format!("first re-parse failed: {e}")))?;
    let again = parser
        .parse(&serializer.to_source(&normal))
        .map_err(|e| TestCaseError::fail(format!("second re-parse failed: {e}")))?;
    prop_assert_eq!(&normal, &again, "normal form is not a fixpoint");
    Ok(())
}

/// A representative document exercising every block and inline kind, including
/// the GFM extensions and both component syntaxes.
const KITCHEN_SINK: &str = "# Heading *one*\n\n\
Paragraph with **bold**, *em*, ~~strike~~, `code`, a [link](https://example.com \"t\"), and text[^n].\n\n\
> a block quote\n> over two lines\n\n\
- bullet a\n- bullet b\n\n\
1. first\n2. second\n\n\
- [x] done\n- [ ] todo\n\n\
| Feature | Status |\n|:---|---:|\n| Tables | yes |\n\n\
```rust\nfn main() {}\n```\n\n\
:::callout{type=warning}\nHeads up.\n:::\n\n\
::::tabs\n:::tab{title=Overview}\nFirst.\n:::\n::::\n\n\
```chart kind=bar\nmonth,sales\njan,10\n```\n\n\
[^n]: the footnote body.\n";

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Deterministic strong round-trip: serialise a parsed document, re-parse, and
/// assert the AST is unchanged. Used for realistic documents already in normal
/// form. Returns `Err` if either parse fails; asserts on AST mismatch.
fn assert_round_trips(source: &str) -> TestResult {
    let parser = CommonMarkParser::new();
    let serializer = MarkdownSerializer::new();
    let first = parser.parse(source)?;
    let reserialised = serializer.to_source(&first);
    let second = parser.parse(&reserialised)?;
    assert_eq!(
        first, second,
        "round-trip changed the AST\n--- original ---\n{source}\n--- reserialised ---\n{reserialised}"
    );
    Ok(())
}

#[test]
fn kitchen_sink_round_trips() -> TestResult {
    assert_round_trips(KITCHEN_SINK)
}

#[test]
fn each_feature_round_trips() -> TestResult {
    let cases = [
        "# h",
        "plain para",
        "a *b* **c** ~~d~~ `e`",
        "1 < 2 & 3 > 0 with a # hash and * star",
        "> quote",
        "- a\n- b",
        "3. c\n4. d",
        "- [x] yes\n- [ ] no",
        "| a | b |\n|---|---|\n| 1 | 2 |",
        "```\ncode\n```",
        "---",
        ":::callout\nhi\n:::",
        ":::tabs\n:::tab{title=X}\ny\n:::\n:::",
        "```chart kind=line\nm,v\n```",
        "text[^1]\n\n[^1]: note",
        "[a](http://e.com) and ![img](http://e.com/i.png)",
    ];
    for case in cases {
        assert_round_trips(case)?;
    }
    Ok(())
}

fn fragment() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("# heading"),
        Just("paragraph text"),
        Just("a *b* **c** ~~d~~ `e` f"),
        Just("1 < 2 & 3"),
        Just("> quote line"),
        Just("- bullet"),
        Just("1. ordered"),
        Just("- [x] done"),
        Just("- [ ] todo"),
        Just("| a | b |\n|---|---|\n| 1 | 2 |"),
        Just("```\ncode\n```"),
        Just("---"),
        Just(":::callout{type=note}"),
        Just(":::"),
        Just("```chart kind=bar\nm,v\n```"),
        Just("text[^a]"),
        Just("[^a]: note"),
        Just("[link](https://example.com)"),
        Just(""),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    #[test]
    fn structured_documents_round_trip(parts in vec(fragment(), 0..40)) {
        round_trip_is_stable(&parts.join("\n\n"))?;
    }
}
