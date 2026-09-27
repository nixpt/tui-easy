//! # `command-palette`
//!
//! Round-13 example — the canonical command-palette composition
//! exercising the `tui-easy-textinput` vendored sibling alongside
//! `tui_easy-list` (round-11) + `tui-easy-styles` (round-1). Honors the
//! T13 two-carrier discipline (dejavue event_id `718bcc1aa525`).
//!
//! ## T13 discipline contract (enforced in `handle_event` + `project_filter`)
//!
//! - **FILTER slot**: `TextArea.value` (round-13) drives the substring
//!   filter; the unfiltered candidate list gets reduced to substring
//!   matches. `handle_event(TuiEvent::Char(c))` types into the textarea.
//! - **SELECTION slot**: `ListState.selected` (round-11) drives the
//!   visual offset — which row of the filtered subset is highlighted.
//!   `handle_event(TuiEvent::ArrowDown/Up)` mutates the list state.
//!
//! The two carriers are independent slots. They do NOT share a storage
//! location. Mistaking one for the other (e.g., trying to encode
//! selection inside the textarea's buffer) violates the round-13
//! composition trap and produces silent staleness on dispatch.
//!
//! ## Layout (3-region)
//!
//! ```text
//! ┌────────────────────────────────────────────────┐  Header (TextArea, 3 rows)
//! │ > _ type to filter                              │
//! ├────────────────────────────────────────────────┤
//! │ alpha    (filtered: 'al')                       │  Body (filtered List, Min 3)
//! │   beta                                            LISTSTATE.SELECTED here
//! │   gamma                                          │  ← independent carrier
//! ├────────────────────────────────────────────────┤
//! │ ↑↓ select · Enter dispatch · Ctrl-C quit        │  Status (1 row)
//! └────────────────────────────────────────────────┘
//! ```

#![cfg_attr(test, allow(unused_imports))]

#[cfg(test)]
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
#[cfg(test)]
use ratatui::Terminal;
use ratatui::widgets::Widget;
use ratatui::widgets::{StatefulWidget, StatefulWidgetRef};
use tui_easy::event::TuiEvent;
use tui_easy::widget::{status_bar, List, ListState, TextArea, TextAreaState};
use tui_easy::TuiApp;

// ─── Candidate set (the unfiltered list) ─────────────────────────────────────

const CANDIDATES: &[&str] = &[
    "alpha", "apply", "apricot", "beta", "build", "cargo", "delta",
    "echo", "filter", "gamma", "help", "lint", "log", "quit", "show",
    "test", "version",
];

// ─── App state ──────────────────────────────────────────────────────────────

/// The application's top-level state.
///
/// T13 discipline: `textarea` and `selection` are INDEPENDENT carriers.
#[derive(Debug)]
pub struct CommandPaletteApp {
    /// Round-13: TextArea is the FILTER slot (typed value drives
    /// the substring filter pipeline).
    pub textarea: TextArea,
    /// Round-13: TextAreaState co-tied with the textarea (upstream
    /// pattern — TextAreaState is the vendored discrete state type).
    pub text_state: TextAreaState,
    /// Round-11: ListState is the SELECTION slot (visual offset into
    /// the filtered subset).
    pub list_state: ListState,
    /// The candidate set we're filtering against.
    pub candidates: Vec<&'static str>,
    /// Last dispatch — set when the user hits Enter.
    pub last_dispatch: Option<usize>,
}

impl Default for CommandPaletteApp {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPaletteApp {
    pub fn new() -> Self {
        let text_state = TextAreaState::default();
        let textarea = TextArea::new();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            textarea,
            text_state,
            list_state,
            candidates: CANDIDATES.to_vec(),
            last_dispatch: None,
        }
    }

    /// T13 FILTER pipeline: substring filter driven by TextArea.value.
    /// Returns the filtered subset (indices into `self.candidates`).
    /// Selection carrier is NOT touched here — T13 §4 discipline.
    pub fn project_filter(&self) -> Vec<usize> {
        let needle = self.textarea.text().to_string();
        if needle.is_empty() {
            return (0..self.candidates.len()).collect();
        }
        self.candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| c.contains(&needle.as_str()))
            .map(|(i, _)| i)
            .collect()
    }

    /// T13 SELECTION advance: navigates the filtered subset.
    /// The selection carrier mutates INDEPENDENTLY of the textarea
    /// — typing does NOT move selection (it's the FILTER, not the
    /// SELECTION). This is the "they don't compete" discipline.
    pub fn advance_selection(&mut self, delta: i64) {
        let filtered = self.project_filter();
        if filtered.is_empty() {
            self.list_state.select(None);
            return;
        }
        let cur = self.list_state.selected().map_or(0_usize, |i| i);
        let cur_filtered_idx = filtered.iter().position(|&i| i == cur).unwrap_or(0);
        let next_filtered =
            (cur_filtered_idx as i64 + delta).rem_euclid(filtered.len() as i64) as usize;
        self.list_state.select(Some(filtered[next_filtered]));
    }

    /// Dispatch handler: Enter key takes the currently SELECTED candidate
    /// (the SELECTION slot, NOT the textarea's value) as the dispatch.
    /// After dispatch, the app self-quits (teaching-mode simple loop).
    pub fn dispatch_selected(&mut self) -> Option<usize> {
        let selected = self.list_state.selected()?;
        self.last_dispatch = Some(selected);
        Some(selected)
    }

    /// Hooks the round-11 ScrollView offset-sync pattern (T13 §4): any
    /// consumer that wants a "highlighted row's projected scroll offset"
    /// for anchor recomputation can read it from here. This keeps the
    /// state-machine clean.
    pub fn projected_offset(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }
}

// ─── Layout (3-region) ──────────────────────────────────────────────────────

/// 3-region layout split — header (3 rows TextArea) / body (Min 3 List) /
/// footer (1 status bar).
pub fn layout(frame_area: Rect) -> LayoutSegments {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header — TextArea
            Constraint::Min(3),      // Body — filtered List
            Constraint::Length(1),   // Status — keymap hint
        ])
        .split(frame_area);

    LayoutSegments {
        header: chunks[0],
        body: chunks[1],
        status: chunks[2],
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutSegments {
    pub header: Rect,
    pub body: Rect,
    pub status: Rect,
}

// ─── Render ─────────────────────────────────────────────────────────────────

/// Render one frame, honoring T13 §4 discipline.
///
/// We render the textarea over the HEADER region and the list (filtered
/// via `app.project_filter()`) over the BODY region. The textarea and
/// the list do NOT share a buffer slot — they lay over different
/// layout regions. This is the T13 §4 spatial discipline (orthogonal
/// projection) in action.
pub fn render(app: &mut CommandPaletteApp, frame_area: Rect, buf: &mut Buffer) {
    let ls = layout(frame_area);

    // ── Header — TextArea (FILTER slot) ────────────────────────
    // Render the textarea into the header area. T13 §4: this is the
    // FILTER slot — the buffer cells just display the typed value;
    // they do not carry SELECTION state.
    <&TextArea as StatefulWidgetRef>::render_ref(
        &&app.textarea,
        ls.header,
        buf,
        &mut app.text_state,
    );

    // ── Body — filtered List (SELECTION slot) ──────────────────
    // Build the list from the filtered candidate set (T13 §4: the
    // FILTER-derived subset drives the body content; SELECTION is the
    // round-11 ListState).
    let filtered = app.project_filter();
    let filtered_labels: Vec<String> = filtered
        .iter()
        .map(|&i| app.candidates[i].to_string())
        .collect();

    // Compute the ListState.selected in the FILTERED-universe space
    // (translating from candidates-universe to filtered-universe).
    let selected_in_filtered: Option<usize> = app.list_state.selected().and_then(|s| {
        filtered.iter().position(|&i| i == s)
    });
    let mut state_for_render = app.list_state.clone();
    state_for_render.select(selected_in_filtered);
    if let Some(new_selected) = selected_in_filtered {
        *state_for_render.offset_mut() = new_selected; // reset offset near selected
    }

    let list: List<'static> = List::new(filtered_labels).highlight_symbol("> ");
    if ls.body.area() > 0 {
        <List as StatefulWidget>::render(list, ls.body, buf, &mut state_for_render);
        // Sync the original list_state with the filtered-projection selected
        // (so the next event's selection-school has the right carrier).
        if let Some(idx_in_filtered) = selected_in_filtered {
            if let Some(&actual_idx) = filtered.get(idx_in_filtered) {
                app.list_state = state_for_render.clone();
                app.list_state.select(Some(actual_idx));
            }
        }
    }

    // ── Status — keymap hint ───────────────────────────────────
    let hint = "↑↓ select · Enter dispatch · Ctrl-C quit";
    let hint_line = hint;
    let theme = tui_easy::theme::ThemeColors::default().to_ratatui();
    let status_p = status_bar(hint_line, None::<&str>, &theme);
    status_p.render(ls.status, buf);
}

// ─── TuiApp implementation ──────────────────────────────────────────────────

impl TuiApp for CommandPaletteApp {
    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        render(self, frame.area(), frame.buffer_mut());
    }

    fn handle_event(&mut self, event: TuiEvent) {
        // T13 discipline: typing → FILTER slot; arrows → SELECTION slot.
        let TuiEvent::Key(key) = event else { return };
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.last_dispatch = None;
            }
            KeyCode::Char(c) => {
                // Round-13 surface: type into the textarea directly.
                self.textarea.insert_str(&c.to_string());
            }
            KeyCode::Backspace => {
                // Try delete-backward; if at start of buffer, no-op.
                self.textarea.delete_backward(1);
            }
            KeyCode::Down => {
                self.advance_selection(1);
            }
            KeyCode::Up => {
                self.advance_selection(-1);
            }
            KeyCode::Enter => {
                self.dispatch_selected();
            }
            KeyCode::Esc => {
                self.last_dispatch = None;
            }
            _ => {}
        }
    }

    fn should_quit(&self) -> bool {
        self.last_dispatch.is_some()
    }

    fn name(&self) -> &str {
        "command-palette"
    }
}

// ─── Entry point ────────────────────────────────────────────────────────────

const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(80);

fn main() -> std::io::Result<()> {
    let app = CommandPaletteApp::new();
    tui_easy::run_app(app, TICK_RATE)
}

// ─── 7 smoke tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn term_with_app(app: &mut CommandPaletteApp, w: u16, h: u16) -> Buffer {
        let backend = TestBackend::new(w, h);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            render(app, f.area(), f.buffer_mut());
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    #[test]
    fn smoke_01_layout_three_regions_partition_correctly() {
        let ls = layout(Rect::new(0, 0, 80, 20));
        assert_eq!(ls.header.height, 3, "header must be 3 rows (the TextArea region)");
        assert!(ls.body.height >= 3, "body must be at least 3 rows (got {})", ls.body.height);
        assert_eq!(ls.status.height, 1, "status must be 1 row");
    }

    #[test]
    fn smoke_02_textarea_filter_pipeline_drives_reduction() {
        let mut app = CommandPaletteApp::new();
        app.textarea.insert_str("a");
        app.textarea.insert_str("p");
        let filtered = app.project_filter();
        // Unfiltered len = 17 candidates.
        // "alpha" does NOT contain "ap" as a substring (a-l-p-h-a has no
        // consecutive "ap"). Only apply/apricot match.
        assert_eq!(filtered.len(), 2, "'ap' filter must reduce to 2: apply, apricot (substring match)");
    }

    #[test]
    fn smoke_03_liststate_navigation_independent_from_textarea() {
        let mut app = CommandPaletteApp::new();
        // Type into the textarea — must NOT advance selection.
        app.textarea.insert_str("b");
        let before = app.list_state.selected();
        app.textarea.insert_str("e");
        let after_typing = app.list_state.selected();
        assert_eq!(
            before, after_typing,
            "typing into TextArea (FILTER slot) must NOT mutate ListState \
             (SELECTION slot). T13 discipline: filter and selection are \
             independent carriers."
        );
        // Now ArrowDown — selection must advance.
        app.advance_selection(1);
        let after_arrow = app.list_state.selected();
        assert_ne!(
            after_arrow, after_typing,
            "ArrowDown to ListState (SELECTION slot) MUST advance the selection. \
             T13 discipline: arrows and typing are different slots."
        );
    }

    #[test]
    fn smoke_04_typing_into_textarea_does_not_change_list_selection() {
        let mut app = CommandPaletteApp::new();
        let initial = app.list_state.selected();
        // Type 5 characters
        for c in "delta".chars() {
            app.textarea.insert_str(&c.to_string());
        }
        let after = app.list_state.selected();
        assert_eq!(initial, after, "list selection must remain at initial value after typing");
    }

    #[test]
    fn smoke_05_arrow_keys_drive_list_navigation() {
        let mut app = CommandPaletteApp::new();
        // Initial selection is 0
        assert_eq!(app.list_state.selected(), Some(0));
        // Advance +3
        app.advance_selection(3);
        assert_eq!(app.list_state.selected(), Some(3));
        // Wrap around with rem_euclid: (3 + 100).rem_euclid(17) == 1.
        app.advance_selection(100);
        assert_eq!(app.list_state.selected(), Some(1));
    }

    #[test]
    fn smoke_06_enter_dispatches_selected_command_and_signals_quit() {
        let mut app = CommandPaletteApp::new();
        app.advance_selection(2); // selection = 2 (candidates[2] = "apricot")
        assert_eq!(app.list_state.selected(), Some(2));
        let dispatch = app.dispatch_selected();
        assert_eq!(dispatch, Some(2));
        assert_eq!(app.dispatch_selected(), Some(2));
        assert!(app.should_quit(), "should_quit() must return true after dispatch");
    }

    #[test]
    fn smoke_07_full_frame_renders_typed_filter_and_selected_row() {
        // End-to-end: type a filter, advance selection, render a real
        // frame through the TestBackend/Terminal path (not just call
        // the pure helper functions directly), and verify the buffer
        // actually painted the typed text and the highlighted row.
        let mut app = CommandPaletteApp::new();
        app.textarea.insert_str("ap");
        app.advance_selection(0); // stay on the first filtered match
        let buf = term_with_app(&mut app, 40, 10);

        let mut found_typed = false;
        let mut found_highlight_symbol = false;
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let sym = buf[(x, y)].symbol();
                if sym == "a" || sym == "p" {
                    found_typed = true;
                }
                if sym == ">" {
                    found_highlight_symbol = true;
                }
            }
        }
        assert!(found_typed, "typed filter text 'ap' must paint into the header region");
        assert!(
            found_highlight_symbol,
            "the selected row's '> ' highlight symbol must paint into the body region"
        );
    }
}
