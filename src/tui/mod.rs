pub mod app;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

use self::app::{App, Panel, View};
use workgraph::graph::Status;
use workgraph::AgentStatus;

/// Interval between data refresh polls
const POLL_TIMEOUT: Duration = Duration::from_millis(250);

/// Run the TUI application
pub fn run(workgraph_dir: PathBuf, refresh_rate_ms: u64) -> Result<()> {
    // Set up panic handler that restores the terminal before printing the panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let mut terminal = ratatui::init();
    let mut app = App::new(workgraph_dir, Duration::from_millis(refresh_rate_ms));

    let result = run_event_loop(&mut terminal, &mut app);

    // Always restore terminal, even if event loop errored
    ratatui::restore();

    result
}

/// Main event loop: poll for keyboard input and redraw
fn run_event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        // Auto-refresh data periodically
        app.maybe_refresh();
        app.poll_log_viewer();

        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(POLL_TIMEOUT)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // Help overlay intercepts all keys when shown
                    if app.show_help {
                        match key.code {
                            KeyCode::Char('?') | KeyCode::Esc => app.show_help = false,
                            _ => {} // swallow all other keys while help is shown
                        }
                    } else if key.code == KeyCode::Char('?') {
                        app.show_help = true;
                    } else {
                        match app.view {
                            View::Dashboard => handle_key(app, key.code),
                            View::LogView => handle_log_key(app, key.code),
                            View::GraphExplorer => handle_graph_key(app, key.code),
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal will be redrawn on next iteration; nothing special needed
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Handle a key press in the dashboard view
fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Tab | KeyCode::BackTab => app.toggle_panel(),
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
        KeyCode::Char('r') => app.refresh_all(),
        KeyCode::Char('g') => app.open_graph_explorer(),
        KeyCode::Enter => app.drill_in(),
        _ => {}
    }
}

/// Handle a key press in the graph explorer
fn handle_graph_key(app: &mut App, code: KeyCode) {
    // Check if detail overlay is shown
    let showing_detail = app.graph_explorer.as_ref().map_or(false, |e| e.show_detail);

    if showing_detail {
        match code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Esc | KeyCode::Enter => {
                if let Some(ref mut explorer) = app.graph_explorer {
                    explorer.show_detail = false;
                    explorer.detail_task = None;
                    explorer.detail_scroll = 0;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(ref mut explorer) = app.graph_explorer {
                    explorer.detail_scroll_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref mut explorer) = app.graph_explorer {
                    explorer.detail_scroll_down();
                }
            }
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.close_graph_explorer(),
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut explorer) = app.graph_explorer {
                explorer.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut explorer) = app.graph_explorer {
                explorer.scroll_down();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if let Some(ref mut explorer) = app.graph_explorer {
                explorer.collapse();
            }
            app.refresh_graph_explorer();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if let Some(ref mut explorer) = app.graph_explorer {
                explorer.expand();
            }
            app.refresh_graph_explorer();
        }
        KeyCode::Enter => {
            // If the selected task has an active agent, jump to its log viewer
            let agent_id = app.graph_explorer.as_ref()
                .and_then(|e| e.selected_task_first_agent());
            if let Some(agent_id) = agent_id {
                app.open_log_viewer_for_agent(&agent_id);
            } else {
                let wg_dir = app.workgraph_dir.clone();
                if let Some(ref mut explorer) = app.graph_explorer {
                    explorer.toggle_detail(&wg_dir);
                }
            }
        }
        KeyCode::Char('r') => {
            app.refresh_graph_explorer();
        }
        KeyCode::Char('a') => {
            if let Some(ref mut explorer) = app.graph_explorer {
                explorer.cycle_to_next_agent_task();
            }
        }
        _ => {}
    }
}

/// Handle a key press in the log viewer
fn handle_log_key(app: &mut App, code: KeyCode) {
    // Get viewport height for scroll calculations (estimate; actual is set during draw)
    let viewport_height = 20_usize; // Will be refined in draw; used as fallback

    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.close_log_viewer(),
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut viewer) = app.log_viewer {
                viewer.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut viewer) = app.log_viewer {
                viewer.scroll_down(viewport_height);
            }
        }
        KeyCode::PageUp => {
            if let Some(ref mut viewer) = app.log_viewer {
                viewer.page_up(viewport_height);
            }
        }
        KeyCode::PageDown => {
            if let Some(ref mut viewer) = app.log_viewer {
                viewer.page_down(viewport_height);
            }
        }
        KeyCode::Char('G') => {
            // Jump to bottom and re-enable auto-scroll
            if let Some(ref mut viewer) = app.log_viewer {
                viewer.auto_scroll = true;
            }
        }
        KeyCode::Char('g') => {
            // Jump to top
            if let Some(ref mut viewer) = app.log_viewer {
                viewer.scroll_offset = 0;
                viewer.auto_scroll = false;
            }
        }
        _ => {}
    }
}

/// Draw the full UI
fn draw(frame: &mut Frame, app: &mut App) {
    match app.view {
        View::Dashboard => draw_dashboard(frame, app),
        View::LogView => draw_log_view(frame, app),
        View::GraphExplorer => draw_graph_explorer(frame, app),
    }

    // Draw help overlay on top of everything if active
    if app.show_help {
        draw_help_overlay(frame, &app.view);
    }
}

/// Draw the main dashboard view
fn draw_dashboard(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Top-level vertical split: main area + status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // main panels
            Constraint::Length(2), // status bar (1 line + top border)
        ])
        .split(size);

    // Main area: horizontal split into task list (left) and agent list (right)
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(outer[0]);

    // Task list panel
    draw_task_list(frame, app, panels[0]);

    // Agent list panel
    draw_agent_list(frame, app, panels[1]);

    // Status bar
    draw_status_bar(frame, app, outer[1]);
}

/// Draw the log viewer for an agent
fn draw_log_view(frame: &mut Frame, app: &mut App) {
    let viewer = match app.log_viewer {
        Some(ref mut v) => v,
        None => return,
    };

    let size = frame.area();

    // Layout: header bar (3 lines) + log content + help bar (1 line)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(3),   // log content
            Constraint::Length(1), // help bar
        ])
        .split(size);

    // --- Header bar ---
    let agent = &viewer.agent;
    let status_label = agent_status_label(&agent.status);
    let status_color = agent_status_color(&agent.status);

    let header_line = Line::from(vec![
        Span::styled(" Agent: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(&agent.id, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("  Task: ", Style::default().fg(Color::Cyan)),
        Span::styled(&agent.task_id, Style::default().fg(Color::White)),
        Span::styled("  PID: ", Style::default().fg(Color::Cyan)),
        Span::styled(agent.pid.to_string(), Style::default().fg(Color::White)),
        Span::styled("  Status: ", Style::default().fg(Color::Cyan)),
        Span::styled(status_label, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::styled("  Uptime: ", Style::default().fg(Color::Cyan)),
        Span::styled(&agent.uptime, Style::default().fg(Color::White)),
    ]);

    let header_block = Block::default()
        .title(" Agent Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let header = Paragraph::new(header_line).block(header_block);
    frame.render_widget(header, layout[0]);

    // --- Log content area ---
    // The content area is the inner area minus the block borders
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner_area = content_block.inner(layout[1]);
    let viewport_height = inner_area.height as usize;

    // Apply auto-scroll before rendering
    viewer.apply_auto_scroll(viewport_height);

    // Update scroll for key handlers that used the fallback viewport
    // (re-clamp offset with actual viewport height)
    let max_offset = viewer.lines.len().saturating_sub(viewport_height);
    if viewer.scroll_offset > max_offset {
        viewer.scroll_offset = max_offset;
    }

    // Build visible lines with wrapping
    let width = inner_area.width as usize;
    let mut wrapped_lines: Vec<Line> = Vec::new();
    let start = viewer.scroll_offset;
    let end = (start + viewport_height).min(viewer.lines.len());

    for line in &viewer.lines[start..end] {
        if width == 0 || line.len() <= width {
            wrapped_lines.push(Line::from(Span::raw(line.clone())));
        } else {
            // Wrap long lines
            let mut pos = 0;
            while pos < line.len() {
                let chunk_end = (pos + width).min(line.len());
                wrapped_lines.push(Line::from(Span::raw(line[pos..chunk_end].to_string())));
                pos = chunk_end;
            }
        }
    }

    let log_content = Paragraph::new(wrapped_lines)
        .block(content_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(log_content, layout[1]);

    // --- Help bar ---
    let scroll_indicator = if viewer.auto_scroll {
        "AUTO-SCROLL"
    } else {
        "PAUSED"
    };
    let line_info = format!(
        " [{scroll_indicator}] Line {}/{} ",
        viewer.scroll_offset + 1,
        viewer.lines.len()
    );
    let help_bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " Log Viewer ",
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(line_info, Style::default().fg(Color::White)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            " q=quit ?=help Esc=back j/k=scroll PgUp/PgDn g=top G=bottom ",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(help_bar, layout[2]);
}

/// Draw the graph explorer view
fn draw_graph_explorer(frame: &mut Frame, app: &mut App) {
    let explorer = match app.graph_explorer {
        Some(ref e) => e,
        None => return,
    };

    let size = frame.area();

    // Layout: graph tree + help bar
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // graph content
            Constraint::Length(1), // help bar
        ])
        .split(size);

    let block = Block::default()
        .title(" Graph Explorer ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if explorer.rows.is_empty() {
        let content = Paragraph::new(vec![
            Line::from(""),
            Line::from("  No tasks found."),
            Line::from("  Load a graph.jsonl into .workgraph/"),
        ])
        .block(block);
        frame.render_widget(content, layout[0]);
    } else {
        let items: Vec<ListItem> = explorer
            .rows
            .iter()
            .map(|row| {
                let indent = "  ".repeat(row.depth);
                let indicator = status_indicator(&row.status);
                let base_color = status_color(&row.status);
                let has_active_agent = row.active_agent_count > 0;

                // Wavefront coloring: active agents get bright style,
                // in-progress without agent gets warm color,
                // done/abandoned get dimmed, pending stays neutral
                let style = if has_active_agent && row.back_ref.is_none() {
                    // Active agent: bright green with bold (the wavefront)
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else if row.critical {
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD)
                } else if row.back_ref.is_some() {
                    Style::default().fg(Color::DarkGray)
                } else if matches!(row.status, Status::InProgress) {
                    // In-progress but no active agent: yellow wavefront edge
                    Style::default().fg(Color::Yellow)
                } else if matches!(row.status, Status::Done | Status::Abandoned) {
                    // Completed: dimmed to show work is behind the wavefront
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(base_color)
                };

                // Build the tree connector
                let connector = if row.depth > 0 {
                    if row.back_ref.is_some() {
                        "↱ "
                    } else {
                        "├─"
                    }
                } else {
                    ""
                };

                // Collapse indicator
                let collapse_marker = if row.back_ref.is_some() {
                    ""
                } else if row.collapsed {
                    "▸ "
                } else {
                    "▾ "
                };

                let mut spans = vec![
                    Span::styled(
                        format!(" {}{}{}", indent, connector, collapse_marker),
                        Style::default().fg(Color::DarkGray),
                    ),
                ];

                // Agent activity marker: pulsing dot before the status indicator
                if has_active_agent && row.back_ref.is_none() {
                    spans.push(Span::styled(
                        "● ",
                        Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
                    ));
                }

                spans.push(Span::styled(format!("{} ", indicator), style));
                spans.push(Span::styled(row.title.clone(), style));

                // Show assigned agent or active agent IDs
                if has_active_agent && row.back_ref.is_none() {
                    let agent_label = if row.active_agent_count > 1 {
                        format!("  [{}x agents: {}]",
                            row.active_agent_count,
                            row.active_agent_ids.join(", "))
                    } else {
                        format!("  [{}]", row.active_agent_ids.first().unwrap_or(&String::new()))
                    };
                    spans.push(Span::styled(
                        agent_label,
                        Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
                    ));
                } else if let Some(ref agent) = row.assigned {
                    spans.push(Span::styled(
                        format!("  ({})", agent),
                        if row.critical {
                            Style::default().fg(Color::LightRed)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ));
                }

                if row.back_ref.is_some() {
                    spans.push(Span::styled(
                        " ↗".to_string(),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let highlight_style = Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);

        let list = List::new(items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol("▶ ");

        let mut state = ListState::default();
        state.select(Some(explorer.selected));

        frame.render_stateful_widget(list, layout[0], &mut state);
    }

    // Help bar
    let has_active = explorer.agent_active_indices.len();
    let agent_hint = if has_active > 0 {
        format!(" a=next agent({})", has_active)
    } else {
        String::new()
    };
    let help_bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " Graph Explorer ",
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" q=quit ?=help Esc=back j/k=nav h/l=fold Enter=details r=refresh{} ", agent_hint),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(help_bar, layout[1]);

    // Draw detail overlay if active
    if explorer.show_detail {
        draw_graph_detail_overlay(frame, explorer);
    }
}

/// Draw the detail overlay for a task in the graph explorer
fn draw_graph_detail_overlay(frame: &mut Frame, explorer: &app::GraphExplorer) {
    let task = match &explorer.detail_task {
        Some(t) => t,
        None => return,
    };

    let size = frame.area();
    // Center overlay: 80% width, 80% height
    let width = (size.width as f32 * 0.8) as u16;
    let height = (size.height as f32 * 0.8) as u16;
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    // Clear the area behind the overlay
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" {} ", task.title))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);

    // Build detail lines
    let mut lines: Vec<Line> = Vec::new();

    // ID and status
    lines.push(Line::from(vec![
        Span::styled(
            "ID: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&task.id, Style::default().fg(Color::White)),
        Span::raw("   "),
        Span::styled(
            "Status: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:?}", task.status),
            Style::default().fg(status_color(&task.status)),
        ),
    ]));

    if let Some(ref agent) = task.assigned {
        lines.push(Line::from(vec![
            Span::styled(
                "Assigned: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(agent.clone(), Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));

    // Description
    if let Some(ref desc) = task.description {
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for desc_line in desc.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", desc_line),
                Style::default().fg(Color::White),
            )));
        }
        lines.push(Line::from(""));
    }

    // Blockers
    if !task.blocked_by.is_empty() {
        lines.push(Line::from(Span::styled(
            "Blocked by:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for blocker in &task.blocked_by {
            lines.push(Line::from(Span::styled(
                format!("  - {}", blocker),
                Style::default().fg(Color::Red),
            )));
        }
        lines.push(Line::from(""));
    }

    // Blocks
    if !task.blocks.is_empty() {
        lines.push(Line::from(Span::styled(
            "Blocks:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for blocked in &task.blocks {
            lines.push(Line::from(Span::styled(
                format!("  - {}", blocked),
                Style::default().fg(Color::Yellow),
            )));
        }
        lines.push(Line::from(""));
    }

    // Artifacts
    if !task.artifacts.is_empty() {
        lines.push(Line::from(Span::styled(
            "Artifacts:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for artifact in &task.artifacts {
            lines.push(Line::from(Span::styled(
                format!("  - {}", artifact),
                Style::default().fg(Color::Green),
            )));
        }
        lines.push(Line::from(""));
    }

    // Deliverables
    if !task.deliverables.is_empty() {
        lines.push(Line::from(Span::styled(
            "Deliverables:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for d in &task.deliverables {
            lines.push(Line::from(Span::styled(
                format!("  - {}", d),
                Style::default().fg(Color::White),
            )));
        }
        lines.push(Line::from(""));
    }

    // Log entries (most recent first)
    if !task.log.is_empty() {
        lines.push(Line::from(Span::styled(
            "Log:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for entry in task.log.iter().rev().take(20) {
            let actor_str = entry
                .actor
                .as_ref()
                .map(|a| format!(" [{}]", a))
                .unwrap_or_default();
            lines.push(Line::from(Span::styled(
                format!("  {}{}: {}", entry.timestamp, actor_str, entry.message),
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines.push(Line::from(""));
    }

    // Failure reason
    if let Some(ref reason) = task.failure_reason {
        lines.push(Line::from(Span::styled(
            "Failure reason:",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", reason),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
    }

    // Apply scroll
    let viewport_height = inner.height as usize;
    let max_scroll = lines.len().saturating_sub(viewport_height);
    let scroll = explorer.detail_scroll.min(max_scroll);
    let visible_lines: Vec<Line> = lines.into_iter().skip(scroll).take(viewport_height).collect();

    let paragraph = Paragraph::new(visible_lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Draw the task list panel with status indicators and color coding
fn draw_task_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.selected_panel == Panel::Tasks;
    let border_style = panel_style(app, Panel::Tasks);

    let title = format!(" Tasks ({}) ", app.task_counts.total);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.tasks.is_empty() {
        let content = Paragraph::new(vec![
            Line::from(""),
            Line::from("  No tasks found."),
            Line::from("  Load a graph.jsonl into .workgraph/"),
        ])
        .block(block);
        frame.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|entry| {
            let highlighted = app.is_task_highlighted(&entry.id);
            let indicator = status_indicator(&entry.status);
            let base_color = status_color(&entry.status);

            // Recently changed items get a bright magenta background flash
            let style = if highlighted {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(base_color)
            };

            let mut spans = vec![
                Span::styled(format!(" {} ", indicator), style),
                Span::styled(entry.title.clone(), style),
            ];

            if let Some(ref agent) = entry.assigned {
                let assign_style = if highlighted {
                    style
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(format!("  ({})", agent), assign_style));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let highlight_style = if is_focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray)
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.task_selected));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Draw the agent list panel with status color coding and PID liveness
fn draw_agent_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.selected_panel == Panel::Agents;
    let border_style = panel_style(app, Panel::Agents);

    let (alive, dead, total) = app.agent_counts;
    let title = format!(" Agents ({} alive / {} dead / {} total) ", alive, dead, total);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.agents.is_empty() {
        let content = Paragraph::new(vec![
            Line::from(""),
            Line::from("  No agents registered."),
            Line::from("  Use 'wg spawn' or 'wg service start' to launch agents."),
        ])
        .block(block);
        frame.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = app
        .agents
        .iter()
        .map(|agent| {
            let highlighted = app.is_agent_highlighted(&agent.id);
            let base_color = agent_status_color(&agent.status);
            let status_label = agent_status_label(&agent.status);

            // Recently changed agents get a bright magenta background flash
            let style = if highlighted {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(base_color)
            };

            let detail_style = if highlighted {
                style
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let spans = vec![
                Span::styled(
                    format!(" {:>9} ", agent.id),
                    style,
                ),
                Span::styled(
                    format!("{:<8} ", status_label),
                    if highlighted { style } else { Style::default().fg(base_color).add_modifier(Modifier::BOLD) },
                ),
                Span::styled(
                    agent.task_id.clone(),
                    style,
                ),
                Span::styled(
                    format!("  {} ", agent.executor),
                    detail_style,
                ),
                Span::styled(
                    format!("pid:{} ", agent.pid),
                    detail_style,
                ),
                Span::styled(
                    agent.uptime.clone(),
                    detail_style,
                ),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let highlight_style = if is_focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray)
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.agent_selected));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Map agent status to a color: green=working, yellow=starting, red=dead, dim=done
fn agent_status_color(status: &AgentStatus) -> Color {
    match status {
        AgentStatus::Working => Color::Green,
        AgentStatus::Starting => Color::Yellow,
        AgentStatus::Idle => Color::Cyan,
        AgentStatus::Stopping => Color::Yellow,
        AgentStatus::Dead => Color::Red,
        AgentStatus::Failed => Color::Red,
        AgentStatus::Done => Color::DarkGray,
    }
}

/// Map agent status to a short label
fn agent_status_label(status: &AgentStatus) -> &'static str {
    match status {
        AgentStatus::Working => "WORKING",
        AgentStatus::Starting => "STARTING",
        AgentStatus::Idle => "IDLE",
        AgentStatus::Stopping => "STOPPING",
        AgentStatus::Dead => "DEAD",
        AgentStatus::Failed => "FAILED",
        AgentStatus::Done => "DONE",
    }
}

/// Map task status to a bracket indicator
fn status_indicator(status: &Status) -> &'static str {
    match status {
        Status::Done => "[x]",
        Status::InProgress => "[~]",
        Status::Open => "[ ]",
        Status::Failed => "[!]",
        Status::Blocked => "[B]",
        Status::PendingReview => "[?]",
        Status::Abandoned => "[-]",
    }
}

/// Map task status to a display color
fn status_color(status: &Status) -> Color {
    match status {
        Status::Done => Color::Green,
        Status::InProgress => Color::Yellow,
        Status::Open => Color::White,
        Status::Failed => Color::Red,
        Status::Blocked => Color::DarkGray,
        Status::PendingReview => Color::Cyan,
        Status::Abandoned => Color::DarkGray,
    }
}

/// Draw the status bar at the bottom of the screen
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let c = &app.task_counts;
    let (alive, _dead, total_agents) = app.agent_counts;
    let service_running = app.is_service_running();

    let status_block = Block::default().borders(Borders::TOP);
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.view_label()),
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} tasks ({} active, {} done) ", c.total, c.in_progress, c.done),
            Style::default().fg(Color::White),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} agents ({} alive) ", total_agents, alive),
            Style::default().fg(Color::White),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            if service_running { " service: running " } else { " service: stopped " },
            Style::default().fg(if service_running { Color::Green } else { Color::DarkGray }),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", app.last_refresh_display),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", app.key_hints()),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(status_block);
    frame.render_widget(status, area);
}

/// Draw the help overlay showing all keybindings
fn draw_help_overlay(frame: &mut Frame, current_view: &View) {
    let size = frame.area();
    // Center overlay: max 60 wide, max 28 tall
    let width = 60.min(size.width.saturating_sub(4));
    let height = 28.min(size.height.saturating_sub(4));
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let inner = block.inner(area);

    let heading = |text: &str| -> Line {
        Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
    };

    let binding = |key: &str, desc: &str| -> Line {
        Line::from(vec![
            Span::styled(
                format!("  {:<14}", key),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(desc.to_string(), Style::default().fg(Color::White)),
        ])
    };

    let blank = || Line::from("");

    let current_label = match current_view {
        View::Dashboard => "Dashboard",
        View::LogView => "Log Viewer",
        View::GraphExplorer => "Graph Explorer",
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Current view: ", Style::default().fg(Color::DarkGray)),
            Span::styled(current_label, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        blank(),
        heading("Global"),
        binding("q", "Quit"),
        binding("?", "Toggle this help"),
        binding("Esc", "Go back / close overlay"),
        blank(),
        heading("Dashboard"),
        binding("Tab", "Switch panel (Tasks / Agents)"),
        binding("j / k", "Navigate up / down"),
        binding("Enter", "Drill into selected item"),
        binding("g", "Open graph explorer"),
        binding("r", "Refresh data"),
        blank(),
        heading("Graph Explorer"),
        binding("j / k", "Navigate up / down"),
        binding("h / l", "Collapse / expand subtree"),
        binding("Enter", "View details or agent log"),
        binding("a", "Cycle to next active agent"),
        binding("r", "Refresh graph"),
        blank(),
        heading("Log Viewer"),
        binding("j / k", "Scroll up / down"),
        binding("PgUp / PgDn", "Page up / down"),
        binding("g / G", "Jump to top / bottom"),
    ];

    // Trim lines to fit viewport
    let max_lines = inner.height as usize;
    lines.truncate(max_lines);

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Return the border style for a panel based on whether it's focused
fn panel_style(app: &App, panel: Panel) -> Style {
    if app.selected_panel == panel {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Restore the terminal to its original state
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
