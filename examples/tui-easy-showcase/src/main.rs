//! # tui-easy-showcase
//!
//! A comprehensive visual demo that exercises **every** widget exposed by the
//! `tui_easy` umbrella crate in one TUI app.  Each tab demonstrates one or more
//! widget types so you can see them side by side (and in the status bar).
//!
//! ## Widgets on display
//!
//! | Widget                    | Where                        |
//! |---------------------------|------------------------------|
//! | `Spinner` (ratatui-cheese)| Title bar                    |
//! | `TabNav` (native Tabs)    | Tab navigation               |
//! | `BigText` (tui-big-text)  | Welcome tab                  |
//! | `Link` (hyperrat)         | Welcome tab                  |
//! | `Sparkline`               | Footer (right flank)         |
//! | `List` + `ListState`      | List tab                     |
//! | `Tree` + `TreeState`      | Tree tab                     |
//! | `Popup` + `PopupState`    | Popup tab                    |
//! | `ScrollView` (via TabLog) | Popup tab (behind popup)     |
//! | `status_bar`              | Footer                       |
//!
//! ## Run it
//!
//! ```sh
//! cargo run -p tui-easy-showcase
//! ```
//!
//! ## Controls
//!
//! | Key           | Action                         |
//! |---------------|--------------------------------|
//! | `Tab` / `←→` | Switch tab                     |
//! | `j` / `k`     | Scroll (List / Tree / Popup)   |
//! | `h` / `l`     | Fold / expand (Tree tab)       |
//! | `p`           | Toggle popup (Popup tab)       |
//! | `q` / `Esc`   | Quit                           |

#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::symbols::bar;

use tui_easy::event::TuiEvent;
use tui_easy::spinner::{SpinnerState, SpinnerType};
use tui_easy::tab_log::TabLog;
use tui_easy::theme::RatatuiThemeColors;
use tui_easy::widget::{
    status_bar, BigText, Link, List, ListState, PixelSize, Popup, PopupState, Sparkline, TabNav,
    Tree, TreeItem, TreeState,
};
use tui_easy::{run_app, TuiApp};

// ── Constants ─────────────────────────────────────────────────────────────

const TICK_RATE: Duration = Duration::from_millis(80);
const SPARKLINE_RING_LEN: usize = 20;
const SCROLL_BUF_LINES: u16 = 120;
const SCROLL_WIDTH: u16 = 80;

/// Tab titles shown in the `TabNav` widget.
const TAB_TITLES: &[&str] = &["Welcome", "Sparkline", "List", "Tree", "Popup"];

// ── App state ─────────────────────────────────────────────────────────────

struct ShowcaseApp {
    /// Active tab index into `TAB_TITLES`.
    active_tab: usize,
    /// Global spinner — rendered in the title bar.
    spinner_state: SpinnerState,
    /// Theme for the status bar.
    theme: RatatuiThemeColors,
    /// Tick bookkeeping.
    last_tick: Instant,
    tick_count: u32,
    /// Rollong-c sparkline ring.
    sparkline_ring: Vec<u64>,
    /// Quit flag.
    quit: bool,

    // ── Widget-specific state ────────────────────────────────────

    /// List widget state.
    list_state: ListState,
    /// Total items in the list (used for index clamping).
    list_len: usize,
    /// Tree widget state.
    tree_state: TreeState<String>,

    /// Popup visibility.
    popup_visible: bool,
    /// Popup state for mouse-drag / positioning.
    popup_state: PopupState,

    /// TabLog for the scrollable content behind the popup.
    popup_log: TabLog,
}

impl ShowcaseApp {
    fn new() -> Self {
        let mut app = Self {
            active_tab: 0,
            spinner_state: SpinnerState::new(SpinnerType::Dot),
            theme: tui_easy::theme::ThemeColors::default().to_ratatui(),
            last_tick: Instant::now(),
            tick_count: 0,
            sparkline_ring: vec![1; SPARKLINE_RING_LEN],
            quit: false,

            // List: start with first item selected.
            list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            list_len: 50,

            // Tree: default closed state.
            tree_state: TreeState::default(),

            popup_visible: false,
            popup_state: PopupState::default(),

            popup_log: TabLog::new(SCROLL_WIDTH, SCROLL_BUF_LINES),
        };

        // Seed the popup log with entries so the scroll view
        // has content even when the popup is closed.
        for i in 0..25 {
            app.popup_log.push_row(
                format!("{:08x}", i * 1000),
                "INFO ".into(),
                format!("log entry #{i} — the quick brown fox jumps over the lazy dog"),
                Some(format!("https://example.com/log/{i}")),
            );
        }
        app.popup_log.rebuild_buffer();

        // Pre-select the root item in the tree.
        app.tree_state.select(vec!["src".to_string()]);

        app
    }

    /// Advance the sparkline ring with a fresh data point.
    fn push_sparkline_metric(&mut self, value: u64) {
        self.sparkline_ring.rotate_left(1);
        let last = self.sparkline_ring.len() - 1;
        self.sparkline_ring[last] = value;
    }

    fn tab_name(&self) -> &str {
        TAB_TITLES[self.active_tab]
    }
}

// ── TuiApp impl ───────────────────────────────────────────────────────────

impl TuiApp for ShowcaseApp {
    fn name(&self) -> &str {
        "tui-easy-showcase"
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn update(&mut self) {
        let dt = self.last_tick.elapsed();
        self.spinner_state.tick(dt);
        self.last_tick = Instant::now();

        self.tick_count = self.tick_count.wrapping_add(1);

        // Push a deterministic sine-wave metric each tick.
        let metric = ((self.tick_count as f64 * 0.3).sin() * 15.0 + 20.0) as u64;
        self.push_sparkline_metric(metric);

        // Keep the popup log pinned to bottom so new entries scroll in.
        self.popup_log.reset_to_bottom_if_pinned();
        if self.tick_count % 5 == 0 {
            self.popup_log.push_row(
                format!("{:08x}", self.tick_count * 1000),
                "INFO ".into(),
                format!("background tick #{}", self.tick_count),
                None,
            );
            self.popup_log.rebuild_buffer();
        }
    }

    fn handle_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Tick | TuiEvent::Resize(_, _) => {}

            // Quit keys.
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            })
            | TuiEvent::Key(KeyEvent {
                code: KeyCode::Esc, ..
            })
            | TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                self.quit = true;
            }

            // ── Tab navigation (global) ─────────────────────────
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Tab, ..
            }) => {
                self.active_tab = (self.active_tab + 1) % TAB_TITLES.len();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::BackTab,
                ..
            }) => {
                self.active_tab = self
                    .active_tab
                    .checked_sub(1)
                    .unwrap_or(TAB_TITLES.len() - 1);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Left, ..
            }) if self.active_tab != 3 => {
                self.active_tab = self
                    .active_tab
                    .checked_sub(1)
                    .unwrap_or(TAB_TITLES.len() - 1);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Right,
                ..
            }) if self.active_tab != 3 => {
                self.active_tab = (self.active_tab + 1) % TAB_TITLES.len();
            }

            // ── Popup tab: toggle ───────────────────────────────
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('p'),
                ..
            }) if self.active_tab == 4 => {
                self.popup_visible = !self.popup_visible;
            }

            // ── List tab: scroll (manual index management) ──────
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                ..
            }) if self.active_tab == 2 => {
                let sel = self.list_state.selected().unwrap_or(0);
                self.list_state
                    .select(Some((sel + 1).min(self.list_len.saturating_sub(1))));
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                ..
            }) if self.active_tab == 2 => {
                let sel = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(sel.saturating_sub(1)));
            }

            // ── Tree tab: navigate ──────────────────────────────
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                ..
            }) if self.active_tab == 3 => {
                self.tree_state.key_down();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                ..
            }) if self.active_tab == 3 => {
                self.tree_state.key_up();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('l') | KeyCode::Right,
                ..
            }) if self.active_tab == 3 => {
                self.tree_state.key_right();
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('h') | KeyCode::Left,
                ..
            }) if self.active_tab == 3 => {
                self.tree_state.key_left();
            }

            // ── Popup tab: scroll behind popup ──────────────────
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                ..
            }) if self.active_tab == 4 && !self.popup_visible => {
                let at_bottom = self.popup_log.scroll_state_mut().is_at_bottom();
                self.popup_log.scroll_state_mut().scroll_down();
                self.popup_log.set_pinned(at_bottom);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                ..
            }) if self.active_tab == 4 && !self.popup_visible => {
                self.popup_log.scroll_state_mut().scroll_up();
                self.popup_log.set_pinned(false);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::End,
                ..
            }) if self.active_tab == 4 && !self.popup_visible => {
                self.popup_log.scroll_state_mut().scroll_to_bottom();
                self.popup_log.set_pinned(true);
            }
            TuiEvent::Key(KeyEvent {
                code: KeyCode::Home,
                ..
            }) if self.active_tab == 4 && !self.popup_visible => {
                self.popup_log.scroll_state_mut().scroll_to_top();
                self.popup_log.set_pinned(false);
            }

            _ => {}
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // Four-layer vertical layout.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),    // Title
                Constraint::Length(3),    // TabNav
                Constraint::Min(3),       // Body
                Constraint::Length(2),    // Footer
            ])
            .split(area);

        let title_area = chunks[0];
        let tabnav_area = chunks[1];
        let body_area = chunks[2];
        let footer_area = chunks[3];

        // ── Title ───────────────────────────────────────────────
        let glyph = self.spinner_state.frame_str();
        let title_text = format!(
            " {glyph} tui-easy-showcase │ {} │ tick {}",
            self.tab_name(),
            self.tick_count,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                title_text,
                Style::default().fg(Color::Cyan),
            ))),
            title_area,
        );

        // ── TabNav ──────────────────────────────────────────────
        let tabs_widget = TabNav::new(
            TAB_TITLES
                .iter()
                .copied()
                .map(Line::from)
                .collect::<Vec<Line<'static>>>(),
        )
        .select(self.active_tab)
        .divider(" │ ")
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(tabs_widget, tabnav_area);

        // ── Body ────────────────────────────────────────────────
        match self.active_tab {
            0 => self.render_welcome_tab(frame, body_area),
            1 => self.render_sparkline_tab(frame, body_area),
            2 => self.render_list_tab(frame, body_area),
            3 => self.render_tree_tab(frame, body_area),
            4 => self.render_popup_tab(frame, body_area),
            _ => {}
        }

        // ── Footer: status_bar + sparkline ──────────────────────
        let sparkline_cols =
            SPARKLINE_RING_LEN.min(footer_area.width.saturating_sub(10) as usize) as u16;
        let footer_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(sparkline_cols)])
            .split(footer_area);

        let left_text = format!(
            "tab {}/{} │ tick {} │ q │ {}",
            self.active_tab + 1,
            TAB_TITLES.len(),
            self.tick_count,
            match self.active_tab {
                2 => "j/k scroll list",
                3 => "j/k ↑↓ · h/l ←→ fold/expand",
                4 if self.popup_visible => "p close popup",
                4 => "p open popup · j/k scroll log",
                _ => "Tab/←→ switch tab",
            },
        );

        frame.render_widget(
            status_bar(left_text, Some(String::new()), &self.theme),
            footer_split[0],
        );

        frame.render_widget(
            Sparkline::default()
                .data(&self.sparkline_ring[..])
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .bar_set(bar::Set {
                    full: "█",
                    seven_eighths: "▇",
                    three_quarters: "▆",
                    five_eighths: "▅",
                    half: "▄",
                    three_eighths: "▃",
                    one_quarter: "▂",
                    one_eighth: "▁",
                    empty: " ",
                }),
            footer_split[1],
        );
    }
}

// ── Tab 0: Welcome (BigText + Links) ──────────────────────────────────────

impl ShowcaseApp {
    fn render_welcome_tab(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),   // BigText
                Constraint::Min(3),      // Description + links
            ])
            .split(area);

        // ── BigText: large pixel rendering of "TUI EASY" ────────
        let big = BigText::builder()
            .lines(vec![Line::from("TUI EASY")])
            .pixel_size(PixelSize::Full)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .build();
        frame.render_widget(big, chunks[0]);

        // ── Description + link widgets ─────────────────────────
        // Split the lower chunk into three rows: description text,
        // GitHub link, and Ratatui link.
        let lower = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(chunks[1]);

        // Description paragraph.
        let desc = Paragraph::new(Text::from(vec![
            Line::from(Span::styled(
                "Welcome to the tui_easy widget showcase!",
                Style::default().fg(Color::Green),
            )),
            Line::from(""),
            Line::from(Span::raw("Browse the tabs above to explore every widget.")),
            Line::from(""),
        ]));
        frame.render_widget(desc, lower[0]);

        // GitHub link widget.
        let gh_link = Link::new("github.com/nixpt/tui-easy", "https://github.com/nixpt/tui-easy")
            .style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .focused(true);
        let gh_area = Rect {
            x: lower[1].x + 2,
            y: lower[1].y,
            width: lower[1].width.saturating_sub(4),
            height: 1,
        };
        frame.render_widget(gh_link, gh_area);

        // Ratatui link widget.
        let rt_link = Link::new("ratatui.rs", "https://ratatui.rs")
            .style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .focused(true);
        let rt_area = Rect {
            x: lower[2].x + 2,
            y: lower[2].y,
            width: lower[2].width.saturating_sub(4),
            height: 1,
        };
        frame.render_widget(rt_link, rt_area);
    }
}

// ── Tab 1: Sparkline ──────────────────────────────────────────────────────

impl ShowcaseApp {
    fn render_sparkline_tab(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(7), Constraint::Length(3)])
            .split(area);

        // Main sparkline — full width with block border.
        let block = Block::default()
            .title(" Live Sparkline ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Sparkline::default()
                .data(&self.sparkline_ring[..])
                .block(block)
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .bar_set(bar::Set {
                    full: "█",
                    seven_eighths: "▇",
                    three_quarters: "▆",
                    five_eighths: "▅",
                    half: "▄",
                    three_eighths: "▃",
                    one_quarter: "▂",
                    one_eighth: "▁",
                    empty: " ",
                }),
            chunks[0],
        );

        // Legend.
        let legend = Line::from(vec![
            Span::styled("  ▁▂▃▄▅▆▇█ ", Style::default().fg(Color::Yellow)),
            Span::raw("Sine-wave metric — updates every tick ("),
            Span::styled(
                format!("{} pts", SPARKLINE_RING_LEN),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" rolling window)"),
        ]);
        frame.render_widget(Paragraph::new(legend), chunks[1]);
    }
}

// ── Tab 2: List ───────────────────────────────────────────────────────────

impl ShowcaseApp {
    fn render_list_tab(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        // Build list items as Text (which implements Into<ListItem>).
        let items: Vec<Text<'static>> = (1..=self.list_len)
            .map(|i| {
                Text::from(Line::from(Span::raw(format!(
                    "Item #{i:02} — {}",
                    match i % 5 {
                        0 => "the quick brown fox jumps over the lazy dog",
                        1 => "lorem ipsum dolor sit amet consectetur",
                        2 => "adipiscing elit sed do eiusmod tempor",
                        3 => "incididunt ut labore et dolore magna aliqua",
                        4 => "ut enim ad minim veniam quis nostrud",
                        _ => unreachable!(),
                    }
                ))))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Scrollable List ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

// ── Tab 3: Tree ───────────────────────────────────────────────────────────

impl ShowcaseApp {
    fn build_mock_tree() -> Vec<TreeItem<'static, String>> {
        vec![
            TreeItem::new(
                "src".to_string(),
                Line::from(Span::styled("src/", Style::default().fg(Color::Cyan))),
                vec![
                    TreeItem::new("main.rs".to_string(), Line::from("  main.rs"), vec![]).unwrap(),
                    TreeItem::new("lib.rs".to_string(), Line::from("  lib.rs"), vec![]).unwrap(),
                    TreeItem::new(
                        "components".to_string(),
                        Line::from(Span::styled(
                            "  components/",
                            Style::default().fg(Color::Cyan),
                        )),
                        vec![
                            TreeItem::new("header.rs".to_string(), Line::from("    header.rs"), vec![]).unwrap(),
                            TreeItem::new("footer.rs".to_string(), Line::from("    footer.rs"), vec![]).unwrap(),
                            TreeItem::new("sidebar.rs".to_string(), Line::from("    sidebar.rs"), vec![]).unwrap(),
                        ],
                    ).unwrap(),
                    TreeItem::new(
                        "utils".to_string(),
                        Line::from(Span::styled(
                            "  utils/",
                            Style::default().fg(Color::Cyan),
                        )),
                        vec![
                            TreeItem::new("helpers.rs".to_string(), Line::from("    helpers.rs"), vec![]).unwrap(),
                            TreeItem::new("parsers.rs".to_string(), Line::from("    parsers.rs"), vec![]).unwrap(),
                        ],
                    ).unwrap(),
                ],
            ).unwrap(),
            TreeItem::new(
                "docs".to_string(),
                Line::from(Span::styled("docs/", Style::default().fg(Color::Cyan))),
                vec![
                    TreeItem::new("README.md".to_string(), Line::from("  README.md"), vec![]).unwrap(),
                    TreeItem::new("API.md".to_string(), Line::from("  API.md"), vec![]).unwrap(),
                    TreeItem::new("DESIGN.md".to_string(), Line::from("  DESIGN.md"), vec![]).unwrap(),
                ],
            ).unwrap(),
            TreeItem::new(
                "Cargo.toml".to_string(),
                Line::from(Span::styled(
                    "Cargo.toml",
                    Style::default().fg(Color::Yellow),
                )),
                vec![],
            ).unwrap(),
            TreeItem::new(
                "README.md".to_string(),
                Line::from(Span::styled(
                    "README.md",
                    Style::default().fg(Color::Yellow),
                )),
                vec![],
            ).unwrap(),
        ]
    }

    fn render_tree_tab(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let items = Self::build_mock_tree();

        let tree = Tree::new(&items).unwrap()
            .block(
                Block::default()
                    .title(" File System Tree ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(tree, area, &mut self.tree_state);
    }
}

// ── Tab 4: Popup + TabLog ─────────────────────────────────────────────────

impl ShowcaseApp {
    fn render_popup_tab(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        // Background: scrollable log content (TabLog).
        let pair = self.popup_log.render_pair();

        // Render the block border first.
        let block = Block::default()
            .title(" Scroll Log (behind popup) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(block, area);

        // Then render ScrollView inside the block's inner area.
        let inner = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });
        frame.render_stateful_widget(pair.view, inner, pair.state);

        // Foreground: popup overlay (if visible).
        if self.popup_visible {
            let popup_body = Text::from(Line::from(vec![
                Span::raw("This is a "),
                Span::styled(
                    "Popup",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" widget!"),
            ]));
            let popup = Popup::new(popup_body)
                .title(" Popup Demo ")
                .style(Style::default().fg(Color::White).bg(Color::Blue))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));

            // Center the popup within the body area.
            let popup_area = centered_rect(area, 55, 35);
            frame.render_stateful_widget(popup, popup_area, &mut self.popup_state);
        }

        // Instruction hint (visible when popup is closed).
        if !self.popup_visible {
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "p",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to open the popup overlay",
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .style(Style::default().bg(Color::Blue));
            let hint_area = Rect {
                x: area.x + 2,
                y: area.y + area.height.saturating_sub(3),
                width: area.width.saturating_sub(4),
                height: 1,
            };
            frame.render_widget(hint, hint_area);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Compute a centered sub-rectangle of the given percentage size.
fn centered_rect(parent: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let w = parent.width * width_pct / 100;
    let h = parent.height * height_pct / 100;
    let x = parent.x + (parent.width - w) / 2;
    let y = parent.y + (parent.height - h) / 2;
    Rect { x, y, width: w, height: h }
}

// ── Boot ──────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let app = ShowcaseApp::new();
    run_app(app, TICK_RATE)
}

// ── Smoke tests (TestBackend, no real terminal) ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use ratatui::Terminal;
    #[cfg(test)]
    use ratatui::backend::TestBackend;
    #[cfg(test)]
    use ratatui::buffer::Buffer;

    fn fresh_app() -> ShowcaseApp {
        ShowcaseApp::new()
    }

    /// Render the app once into a `TestBackend` of the given size
    /// and return the buffer for cell-by-cell assertions.
    fn render_once(app: &mut ShowcaseApp, w: u16, h: u16) -> Buffer {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| app.draw(frame))
            .expect("draw into TestBackend");
        terminal.backend().buffer().clone()
    }

    /// Concatenate the cell symbols of row `y` across the full width.
    fn row_text(buf: &Buffer, y: u16) -> String {
        let mut s = String::new();
        for x in 0..buf.area.width {
            s.push_str(buf[(x, y)].symbol());
        }
        s
    }

    /// Concatenate the cell symbols of rows `[y0, y1)` across the
    /// full width.
    fn rows_text(buf: &Buffer, y0: u16, y1: u16) -> String {
        let mut s = String::new();
        for y in y0..y1 {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
        }
        s
    }

    // ── Render smoke tests ──────────────────────────────────────

    #[test]
    fn smoke_renders_all_regions() {
        let mut app = fresh_app();
        let buf = render_once(&mut app, 120, 30);

        // 1. Title row (y=0) carries the app name and tick counter.
        let title_line = row_text(&buf, 0);
        assert!(
            title_line.contains("tui-easy-showcase"),
            "title row missing app name: {title_line:?}"
        );
        assert!(
            title_line.contains("tick 0"),
            "title row missing tick counter: {title_line:?}"
        );
        assert!(
            title_line.contains("Welcome"),
            "title row missing active tab name: {title_line:?}"
        );

        // 2. TabNav (y=1..=3) carries all five tab titles.
        let tabnav = rows_text(&buf, 1, 4);
        assert!(
            tabnav.contains("Welcome"),
            "TabNav missing 'Welcome': {tabnav:?}"
        );
        assert!(
            tabnav.contains("Sparkline"),
            "TabNav missing 'Sparkline': {tabnav:?}"
        );
        assert!(
            tabnav.contains("List"),
            "TabNav missing 'List': {tabnav:?}"
        );
        assert!(
            tabnav.contains("Tree"),
            "TabNav missing 'Tree': {tabnav:?}"
        );
        assert!(
            tabnav.contains("Popup"),
            "TabNav missing 'Popup': {tabnav:?}"
        );

        // 3. Body (y=4..height-2) contains the Welcome tab content.
        let body = rows_text(&buf, 4, buf.area.height.saturating_sub(2));
        assert!(
            body.contains("Welcome to the tui_easy widget showcase!"),
            "body missing welcome text: {body:?}"
        );
        assert!(
            body.contains("github.com/nixpt/tui-easy"),
            "body missing GitHub link label: {body:?}"
        );
        assert!(
            body.contains("ratatui.rs"),
            "body missing Ratatui link label: {body:?}"
        );

        // 4. Footer (y=height-1) carries tab counter + status info.
        let footer = row_text(&buf, buf.area.height - 1);
        assert!(
            footer.contains("tab 1/5"),
            "footer missing tab counter: {footer:?}"
        );
        assert!(
            footer.contains("tick 0"),
            "footer missing tick counter: {footer:?}"
        );
    }

    #[test]
    fn smoke_tab_switch_changes_body_content() {
        let mut app = fresh_app();

        // Switch to tab 2 (1-indexed) = Sparkline.
        app.active_tab = 1;
        let buf = render_once(&mut app, 120, 30);
        let title = row_text(&buf, 0);
        assert!(
            title.contains("Sparkline"),
            "after switching to tab 1, title must show 'Sparkline': {title:?}"
        );

        // Switch to tab 3 (List).
        app.active_tab = 2;
        let buf = render_once(&mut app, 120, 30);
        let body = rows_text(&buf, 4, buf.area.height.saturating_sub(2));
        assert!(
            body.contains("Scrollable List"),
            "body must show 'Scrollable List' on List tab: {body:?}"
        );

        // Switch to tab 4 (Tree).
        app.active_tab = 3;
        let buf = render_once(&mut app, 120, 30);
        let body = rows_text(&buf, 4, buf.area.height.saturating_sub(2));
        assert!(
            body.contains("File System Tree"),
            "body must show 'File System Tree' on Tree tab: {body:?}"
        );

        // Switch to tab 5 (Popup).
        app.active_tab = 4;
        let buf = render_once(&mut app, 120, 30);
        let body = rows_text(&buf, 4, buf.area.height.saturating_sub(2));
        assert!(
            body.contains("Scroll Log"),
            "body must show 'Scroll Log' on Popup tab: {body:?}"
        );
    }

    // ── Key routing smoke tests ─────────────────────────────────

    #[test]
    fn smoke_quit_keys_set_should_quit() {
        let mut app = fresh_app();

        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
        )));
        assert!(app.should_quit(), "q must set should_quit");

        let mut app = fresh_app();
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        )));
        assert!(app.should_quit(), "Esc must set should_quit");

        let mut app = fresh_app();
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(app.should_quit(), "Ctrl-c must set should_quit");
    }

    #[test]
    fn smoke_tab_navigation_keys_work() {
        let mut app = fresh_app();
        assert_eq!(app.active_tab, 0, "starts on Welcome tab");

        // Tab advances through all 5 tabs and wraps.
        for expected in 1..=4 {
            app.handle_event(TuiEvent::Key(KeyEvent::new(
                KeyCode::Tab,
                KeyModifiers::NONE,
            )));
            assert_eq!(
                app.active_tab, expected,
                "Tab from tab {} should advance to {}",
                expected - 1,
                expected
            );
        }

        // One more Tab wraps to 0.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::NONE,
        )));
        assert_eq!(app.active_tab, 0, "Tab from last tab wraps to 0");

        // BackTab goes backwards.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::BackTab,
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.active_tab, 4,
            "BackTab from tab 0 wraps to last tab"
        );
    }

    // ── Sparkline smoke test ────────────────────────────────────

    #[test]
    fn smoke_sparkline_metrics_advance_on_push() {
        let mut app = fresh_app();
        let initial_first = app.sparkline_ring[0];
        let initial_last = *app.sparkline_ring.last().unwrap();

        for tick in 100u64..105u64 {
            app.push_sparkline_metric(tick);
        }

        let final_last = *app.sparkline_ring.last().unwrap();
        let final_first = app.sparkline_ring[0];

        assert_eq!(
            final_last, 104,
            "ring's last entry must equal the most recent push"
        );
        assert_eq!(
            app.sparkline_ring[SPARKLINE_RING_LEN - 2],
            103,
            "ring's penultimate entry must equal 103"
        );
        assert_eq!(
            final_first, initial_first,
            "ring's index 0 must survive (20 > 5 pushes)"
        );
        assert_ne!(
            (initial_first, initial_last),
            (final_first, final_last),
            "last entry must change after pushes"
        );
    }

    // ── Popup toggle smoke test ──────────────────────────────────

    #[test]
    fn smoke_popup_toggle_on_popup_tab() {
        let mut app = fresh_app();
        app.active_tab = 4;
        assert!(!app.popup_visible, "popup starts hidden");

        // `p` toggles popup on.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::NONE,
        )));
        assert!(app.popup_visible, "p must show popup");

        // `p` toggles popup off.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::NONE,
        )));
        assert!(!app.popup_visible, "second p must hide popup");

        // `p` on non-popup tab must not toggle.
        app.active_tab = 0;
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::NONE,
        )));
        assert!(!app.popup_visible, "p on Welcome tab must not toggle popup");
    }

    // ── List scroll smoke test ───────────────────────────────────

    #[test]
    fn smoke_list_scroll_on_list_tab() {
        let mut app = fresh_app();
        app.active_tab = 2;

        let sel = app.list_state.selected();
        assert_eq!(sel, Some(0), "list starts selected at index 0");

        // `j` scrolls down.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.list_state.selected(),
            Some(1),
            "j must scroll list to index 1"
        );

        // `k` scrolls up.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.list_state.selected(),
            Some(0),
            "k must scroll list back to index 0"
        );

        // `k` at top does not underflow.
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.list_state.selected(),
            Some(0),
            "k at index 0 must stay at 0"
        );

        // `j` on non-list tab must not scroll.
        app.active_tab = 0;
        app.handle_event(TuiEvent::Key(KeyEvent::new(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.list_state.selected(),
            Some(0),
            "j on Welcome tab must not scroll list"
        );
    }

    // ── Update smoke test ────────────────────────────────────────

    #[test]
    fn smoke_update_advances_tick_and_sparkline() {
        let mut app = fresh_app();
        assert_eq!(app.tick_count, 0, "starts at tick 0");

        app.update();
        assert_eq!(app.tick_count, 1, "update advances tick to 1");

        app.update();
        assert_eq!(app.tick_count, 2, "update advances tick to 2");

        // After 2 updates, the sparkline ring has 2 new sine-wave values.
        // Since the ring was seeded with 1s, and we push 2 deterministic
        // values on top, the last entry must match the expected formula:
        // ((2.0 * 0.3).sin() * 15.0 + 20.0) as u64
        let expected = ((2.0_f64 * 0.3).sin() * 15.0 + 20.0) as u64;
        assert_eq!(
            *app.sparkline_ring.last().unwrap(),
            expected,
            "sparkline ring advances deterministically after 2 ticks"
        );
    }
}
