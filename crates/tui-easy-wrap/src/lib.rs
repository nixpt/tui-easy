//! Word-wrap helpers that operate on `ratatui::text::Line` sequences.
//!
//! Two pieces here:
//!
//! 1. [`word_wrap_line`] / [`word_wrap_lines_borrowed`] — wrap a single or
//!    sequence of [`ratatui::text::Line`]s to a given column width, preserving
//!    per-span styles and supporting optional initial / subsequent indents.
//! 2. [`wrap_ranges`] / [`wrap_ranges_trim`] — raw byte-range helpers giving
//!    where `textwrap::wrap` would slice a `&str`.
//!
//! Both are first-party code from `xai-ratatui-textarea`'s `wrapping.rs`
//! (Apache-2.0). Borrowed from xAI's Grok build (grok-build) crate. Only the
//! reusable surface was lifted out: the test/dev-only helpers in the original
//! (`word_wrap_lines`, which uses an internal `push_owned_lines` from another
//! module) were NOT taken — call [`word_wrap_lines_borrowed`] and `.into_iter()`
//! if you need an owned `Vec<Line<'static>>`.
//!
//! See `LICENSE-APACHE` and `NOTICE` at the root of this crate.

#![allow(clippy::module_name_repetitions)]

use std::ops::Range;

use ratatui::text::{Line, Span};
use textwrap::Options;
use textwrap::wrap_algorithms::Penalties;

// Re-export so callers don't need a direct `textwrap` dep.
pub use textwrap::{LineEnding, WordSeparator, WordSplitter, WrapAlgorithm};

/// Compute byte ranges for a wrapped text, preserving trailing whitespace.
///
/// Each returned range is a half-open byte interval into `text` covering one
/// wrapped output line. The trailing whitespace that follows the last word is
/// included in the range (useful if it carries significance, e.g. styles that
/// key on the trailing space).
pub fn wrap_ranges<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    let mut lines: Vec<Range<usize>> = Vec::new();
    let text_start = text.as_ptr() as usize;
    let text_end = text_start + text.len();
    for line in textwrap::wrap(text, opts).iter() {
        match line {
            std::borrow::Cow::Borrowed(slice) => {
                let slice_addr = slice.as_ptr() as usize;
                // Skip slices whose pointers don't lie within `text`.
                // Guards against empty `Cow::Borrowed("")` slices from
                // textwrap that reference static memory instead of the
                // input buffer (e.g. at zero or degenerate widths).
                if slice_addr < text_start || slice_addr > text_end {
                    continue;
                }
                let start = slice_addr - text_start;
                let end = start + slice.len();
                let trailing_spaces = text[end..].chars().take_while(|c| *c == ' ').count();
                lines.push(start..end + trailing_spaces);
            }
            std::borrow::Cow::Owned(_) => panic!("wrap_ranges: unexpected owned string"),
        }
    }
    lines
}

/// Like [`wrap_ranges`] but returns ranges without trailing whitespace.
pub fn wrap_ranges_trim<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    let mut lines: Vec<Range<usize>> = Vec::new();
    let text_start = text.as_ptr() as usize;
    let text_end = text_start + text.len();
    for line in textwrap::wrap(text, opts).iter() {
        match line {
            std::borrow::Cow::Borrowed(slice) => {
                let slice_addr = slice.as_ptr() as usize;
                if slice_addr < text_start || slice_addr > text_end {
                    continue;
                }
                let start = slice_addr - text_start;
                let end = start + slice.len();
                lines.push(start..end);
            }
            std::borrow::Cow::Owned(_) => panic!("wrap_ranges_trim: unexpected owned string"),
        }
    }
    lines
}

/// Builder-style options for [`word_wrap_line`] / [`word_wrap_lines_borrowed`].
///
/// Wraps `textwrap::Options` with a more ergonomic interface: note that
/// `initial_indent` and `subsequent_indent` are `ratatui::text::Line` (so they
/// can carry style), and `width` is callable as a fluent setter.
#[derive(Debug, Clone)]
pub struct RtOptions<'a> {
    /// The width in columns at which the text will be wrapped.
    pub width: usize,
    /// Line ending used for breaking lines.
    pub line_ending: textwrap::LineEnding,
    /// Indentation used for the first line of output.
    pub initial_indent: Line<'a>,
    /// Indentation used for subsequent lines of output.
    pub subsequent_indent: Line<'a>,
    /// Allow long words to be broken if they cannot fit on a line.
    pub break_words: bool,
    /// Wrapping algorithm; see `textwrap::WrapAlgorithm`.
    pub wrap_algorithm: textwrap::WrapAlgorithm,
    /// Word separator; see `textwrap::WordSeparator`.
    pub word_separator: textwrap::WordSeparator,
    /// Word splitter; see `textwrap::WordSplitter`.
    pub word_splitter: textwrap::WordSplitter,
}

impl From<usize> for RtOptions<'_> {
    fn from(width: usize) -> Self {
        RtOptions::new(width)
    }
}

impl<'a> RtOptions<'a> {
    /// Defaults: width = `width`, optimal-fit algorithm with a near-infinite
    /// overflow penalty (so lines never overflow), and `HyphenSplitter` for
    /// multi-char word breaking.
    pub fn new(width: usize) -> Self {
        RtOptions {
            width,
            line_ending: textwrap::LineEnding::LF,
            initial_indent: Line::default(),
            subsequent_indent: Line::default(),
            break_words: true,
            word_separator: textwrap::WordSeparator::new(),
            wrap_algorithm: textwrap::WrapAlgorithm::OptimalFit(Penalties {
                // ~infinite overflow penalty, we never want to overflow a line.
                overflow_penalty: usize::MAX / 4,
                ..Default::default()
            }),
            word_splitter: textwrap::WordSplitter::HyphenSplitter,
        }
    }

    pub fn line_ending(self, line_ending: LineEnding) -> Self {
        Self { line_ending, ..self }
    }

    pub fn width(self, width: usize) -> Self {
        Self { width, ..self }
    }

    pub fn initial_indent(self, initial_indent: Line<'a>) -> Self {
        Self { initial_indent, ..self }
    }

    pub fn subsequent_indent(self, subsequent_indent: Line<'a>) -> Self {
        Self { subsequent_indent, ..self }
    }

    pub fn break_words(self, break_words: bool) -> Self {
        Self { break_words, ..self }
    }

    pub fn word_separator(self, word_separator: WordSeparator) -> Self {
        Self { word_separator, ..self }
    }

    pub fn wrap_algorithm(self, wrap_algorithm: WrapAlgorithm) -> Self {
        Self { wrap_algorithm, ..self }
    }

    pub fn word_splitter(self, word_splitter: WordSplitter) -> Self {
        Self { word_splitter, ..self }
    }
}

/// Wrap a single `Line` to a max column width, preserving span styles and
/// applying [`RtOptions::initial_indent`] / [`RtOptions::subsequent_indent`].
///
/// Returns a `Vec<Line>` where each element is one wrapped visual line. The
/// `Line`'s `style` field is inherited by every emitted line; per-span styles
/// inside the input are propagated to the corresponding slices in the output.
///
/// `width_or_options` may be either a `usize` (used as the column width) or an
/// [`RtOptions`] (built with `RtOptions::new(w)` and composed fluently).
#[must_use]
pub fn word_wrap_line<'a, O>(line: &'a Line<'a>, width_or_options: O) -> Vec<Line<'a>>
where
    O: Into<RtOptions<'a>>,
{
    // Flatten the line and record span byte ranges.
    let mut flat = String::new();
    let mut span_bounds = Vec::new();
    let mut acc = 0usize;
    for s in &line.spans {
        let text = s.content.as_ref();
        let start = acc;
        flat.push_str(text);
        acc += text.len();
        span_bounds.push((start..acc, s.style));
    }

    let rt_opts: RtOptions<'a> = width_or_options.into();
    let opts = Options::new(rt_opts.width)
        .line_ending(rt_opts.line_ending)
        .break_words(rt_opts.break_words)
        .wrap_algorithm(rt_opts.wrap_algorithm)
        .word_separator(rt_opts.word_separator)
        .word_splitter(rt_opts.word_splitter);

    let mut out: Vec<Line<'a>> = Vec::new();

    // First line: width reduced by initial_indent width.
    let initial_width_available = opts
        .width
        .saturating_sub(rt_opts.initial_indent.width())
        .max(1);
    let initial_wrapped = wrap_ranges_trim(&flat, opts.clone().width(initial_width_available));
    let Some(first_line_range) = initial_wrapped.first() else {
        return vec![rt_opts.initial_indent.clone()];
    };

    let mut first_line = rt_opts.initial_indent.clone().style(line.style);
    {
        let sliced = slice_line_spans(line, &span_bounds, first_line_range);
        let mut spans = first_line.spans;
        spans.append(
            &mut sliced
                .spans
                .into_iter()
                .map(|s| s.patch_style(line.style))
                .collect(),
        );
        first_line.spans = spans;
        out.push(first_line);
    }

    // Subsequent pieces: width reduced by subsequent_indent width.
    let base = first_line_range.end;
    let skip_leading_spaces = flat[base..].chars().take_while(|c| *c == ' ').count();
    let base = base + skip_leading_spaces;
    let subsequent_width_available = opts
        .width
        .saturating_sub(rt_opts.subsequent_indent.width())
        .max(1);
    let remaining_wrapped = wrap_ranges_trim(&flat[base..], opts.width(subsequent_width_available));
    for r in &remaining_wrapped {
        if r.is_empty() {
            continue;
        }
        let mut subsequent_line = rt_opts.subsequent_indent.clone().style(line.style);
        let offset_range = (r.start + base)..(r.end + base);
        let sliced = slice_line_spans(line, &span_bounds, &offset_range);
        let mut spans = subsequent_line.spans;
        spans.append(
            &mut sliced
                .spans
                .into_iter()
                .map(|s| s.patch_style(line.style))
                .collect(),
        );
        subsequent_line.spans = spans;
        out.push(subsequent_line);
    }

    out
}

/// Wrap a sequence of lines. The first input line gets
/// [`RtOptions::initial_indent`]; all later input lines and any wrapped
/// continuations get [`RtOptions::subsequent_indent`]. Borrows from the input
/// `Line`s (output is `Vec<Line<'a>>`).
pub fn word_wrap_lines_borrowed<'a, I, O>(lines: I, width_or_options: O) -> Vec<Line<'a>>
where
    I: IntoIterator<Item = &'a Line<'a>>,
    O: Into<RtOptions<'a>>,
{
    let base_opts: RtOptions<'a> = width_or_options.into();
    let mut out: Vec<Line<'a>> = Vec::new();
    let mut first = true;
    for line in lines {
        let opts = if first {
            base_opts.clone()
        } else {
            base_opts
                .clone()
                .initial_indent(base_opts.subsequent_indent.clone())
        };
        out.extend(word_wrap_line(line, opts));
        first = false;
    }
    out
}

fn slice_line_spans<'a>(
    original: &'a Line<'a>,
    span_bounds: &[(Range<usize>, ratatui::style::Style)],
    range: &Range<usize>,
) -> Line<'a> {
    let start_byte = range.start;
    let end_byte = range.end;
    let mut acc: Vec<Span<'a>> = Vec::new();
    for (i, (s_range, style)) in span_bounds.iter().enumerate() {
        let s = s_range.start;
        let e = s_range.end;
        if e <= start_byte {
            continue;
        }
        if s >= end_byte {
            break;
        }
        let seg_start = start_byte.max(s);
        let seg_end = end_byte.min(e);
        if seg_end > seg_start {
            let local_start = seg_start - s;
            let local_end = seg_end - s;
            let content = original.spans[i].content.as_ref();
            let slice = &content[local_start..local_end];
            acc.push(Span {
                style: *style,
                content: std::borrow::Cow::Borrowed(slice),
            });
        }
        if e >= end_byte {
            break;
        }
    }
    Line {
        style: original.style,
        alignment: original.alignment,
        spans: acc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Stylize};
    use std::string::ToString;

    fn concat_line(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn trivial_unstyled_no_indents_wide_width() {
        let line = Line::from("hello");
        let out = word_wrap_line(&line, 10);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "hello");
    }

    #[test]
    fn simple_unstyled_wrap_narrow_width() {
        let line = Line::from("hello world");
        let out = word_wrap_line(&line, 5);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn simple_styled_wrap_preserves_styles() {
        let line = Line::from(vec!["hello ".red(), "world".into()]);
        let out = word_wrap_line(&line, 6);
        assert_eq!(out.len(), 2);
        // First line carries the red style on the "hello" span.
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(out[0].spans.len(), 1);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Red));
        // Second line is unstyled.
        assert_eq!(concat_line(&out[1]), "world");
        assert_eq!(out[1].spans.len(), 1);
        assert_eq!(out[1].spans[0].style.fg, None);
    }

    #[test]
    fn with_initial_and_subsequent_indents() {
        let opts = RtOptions::new(8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));
        let line = Line::from("hello world foo");
        let out = word_wrap_line(&line, opts);
        let text: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(text[0], "- hello");
        assert_eq!(text[1], "  world");
        assert_eq!(text[2], "  foo");
    }

    #[test]
    fn empty_input_yields_single_empty_line() {
        let line = Line::from("");
        let out = word_wrap_line(&line, 10);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "");
    }

    #[test]
    fn break_words_false_allows_overflow_for_long_word() {
        let opts = RtOptions::new(5).break_words(false);
        let line = Line::from("supercalifragilistic");
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "supercalifragilistic");
    }

    #[test]
    fn hyphen_splitter_breaks_at_hyphen() {
        let line = Line::from("hello-world");
        let out = word_wrap_line(&line, 7);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello-");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn indent_consumes_width_leaving_one_char_space() {
        // Indent ">>>>" (4 cols) + 1 char from a 4-col width = 3 wrapped lines.
        let opts = RtOptions::new(4)
            .initial_indent(Line::from(">>>>"))
            .subsequent_indent(Line::from("--"));
        let line = Line::from("hello");
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 3);
        assert_eq!(concat_line(&out[0]), ">>>>h");
        assert_eq!(concat_line(&out[1]), "--el");
        assert_eq!(concat_line(&out[2]), "--lo");
    }

    #[test]
    fn wide_unicode_wraps_by_display_width() {
        // Each 😀 is 2 display columns. Width 4 -> 2 per line.
        let line = Line::from("😀😀😀");
        let out = word_wrap_line(&line, 4);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "😀😀");
        assert_eq!(concat_line(&out[1]), "😀");
    }

    #[test]
    fn styled_split_within_span_preserves_style() {
        let line = Line::from(vec!["abcd".red()]);
        let out = word_wrap_line(&line, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].spans.len(), 1);
        assert_eq!(out[1].spans.len(), 1);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(out[1].spans[0].style.fg, Some(Color::Red));
        assert_eq!(concat_line(&out[0]), "ab");
        assert_eq!(concat_line(&out[1]), "cd");
    }

    #[test]
    fn wrap_lines_borrowed_applies_initial_indent_only_once() {
        let opts = RtOptions::new(8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));

        let lines = [Line::from("hello world"), Line::from("foo bar baz")];
        let out = word_wrap_lines_borrowed(lines.iter(), opts);

        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert!(rendered.first().unwrap().starts_with("- "));
        for r in rendered.iter().skip(1) {
            assert!(r.starts_with("  "));
        }
    }

    #[test]
    fn wrap_lines_borrowed_without_indents_is_concat_of_single_wraps() {
        let lines = [Line::from("hello"), Line::from("world!")];
        let out = word_wrap_lines_borrowed(lines.iter(), 10);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello", "world!"]);
    }

    #[test]
    fn wrap_ranges_many_newlines_width_one_does_not_panic() {
        let text = "\n".repeat(30);
        let ranges = wrap_ranges(
            &text,
            Options::new(1).wrap_algorithm(WrapAlgorithm::FirstFit),
        );
        for r in &ranges {
            assert!(r.end <= text.len(), "range {r:?} out of bounds");
        }
    }

    #[test]
    fn wrap_ranges_trim_empty_text_does_not_panic() {
        let ranges = wrap_ranges_trim("", 1);
        assert!(ranges.is_empty() || ranges == vec![0..0]);
    }

    #[test]
    fn word_wrap_line_width_one_with_newlines_does_not_panic() {
        let line = Line::from("\n\n\n\n\n");
        let out = word_wrap_line(&line, 1);
        assert!(!out.is_empty());
    }

    #[test]
    fn word_wrap_does_not_split_words_simple_english() {
        let sample = "Years passed, and Willowmere thrived in peace and friendship. Mira’s herb garden flourished with both ordinary and enchanted plants, and travelers spoke of the kindness of the woman who tended them.";
        let line = Line::from(sample);
        let lines = [line];
        let wrapped = word_wrap_lines_borrowed(&lines, 40);
        let joined: String = wrapped.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        assert_eq!(
            joined,
            "Years passed, and Willowmere thrived\nin peace and friendship. Mira’s herb\ngarden flourished with both ordinary and\nenchanted plants, and travelers spoke\nof the kindness of the woman who tended\nthem."
        );
    }
}
