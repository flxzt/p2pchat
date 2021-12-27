use std::io::Stdout;

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

pub trait CycleFocus {
    fn next(self) -> Self;
    fn prev(self) -> Self;
}

use crate::app::{self};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageFocus {
    Chat = 0,
    Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectionPageFocus {
    ConnectionLog = 0,
    RegenerateSwarm,
    AddrInputField,
}

impl CycleFocus for ConnectionPageFocus {
    fn next(self) -> Self {
        match self {
            Self::ConnectionLog => Self::RegenerateSwarm,
            Self::RegenerateSwarm => Self::AddrInputField,
            Self::AddrInputField => Self::ConnectionLog,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::ConnectionLog => Self::AddrInputField,
            Self::RegenerateSwarm => Self::ConnectionLog,
            Self::AddrInputField => Self::RegenerateSwarm,
        }
    }
}

impl CycleFocus for PageFocus {
    fn next(self) -> Self {
        match self {
            Self::Chat => Self::Connection,
            Self::Connection => Self::Chat,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Chat => Self::Connection,
            Self::Connection => Self::Chat,
        }
    }
}

pub struct Ui {
    pub page_focus: PageFocus,
    pub connection_page_focus: ConnectionPageFocus,

    pub chat_input: String,
    pub addr_input: String,
    pub connection_log_allocation: Option<Rect>,
    pub connection_log_liststate: ListState,
}

impl Ui {
    pub fn new() -> Self {
        let mut connection_log_liststate = ListState::default();
        connection_log_liststate.select(Some(0));

        Self {
            page_focus: PageFocus::Chat,
            connection_page_focus: ConnectionPageFocus::AddrInputField,
            chat_input: String::from(""),
            addr_input: String::from(""),
            connection_log_allocation: None,
            connection_log_liststate,
        }
    }
}

pub fn draw_ui(
    app: &mut app::App,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), anyhow::Error> {
    terminal.draw(|frame| {
        let size = frame.size();

        // Surrounding block
        let app_block = Block::default()
            .title(" p2pchat ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        frame.render_widget(app_block, size);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(6)].as_ref())
            .split(size);

        draw_header(frame, chunks[0], app);

        match app.ui.page_focus {
            PageFocus::Chat => {
                draw_chat_page(frame, chunks[1], app);
            }
            PageFocus::Connection => {
                draw_connection_page(frame, chunks[1], app);
            }
        }
    })?;
    Ok(())
}

pub fn draw_header<B: Backend>(frame: &mut Frame<B>, size: Rect, app: &mut app::App) {
    let selected = app.ui.page_focus as usize;

    let titles = ["Chat", "Connection"]
        .iter()
        .cloned()
        .map(Spans::from)
        .collect();
    let pages_tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED),
        )
        .divider(symbols::DOT)
        .select(selected);

    frame.render_widget(pages_tabs, size);
}

pub fn draw_chat_page<B: Backend>(frame: &mut Frame<B>, size: Rect, app: &mut app::App) {
    let chat_page_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
        .split(size);

    // Chat History
    let chat_history_items = app
        .history
        .iter()
        .map(|message| {
            let style = if message.peer_id == *app.connection.swarm.local_peer_id() {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            ListItem::new(Span::styled(format!("{}: {}", message.peer_id, message.text), style))
        })
        .collect::<Vec<ListItem>>();
    let chat_history_list = List::new(chat_history_items)
        .block(
            Block::default()
                .title(Span::styled("History", Style::default()))
                .borders(Borders::ALL),
        );
    frame.render_widget(chat_history_list, chat_page_chunks[0]);

    // Chat Input
    let chat_input_text =
        Text::styled(app.ui.chat_input.clone(), Style::default().fg(Color::White));
    frame.set_cursor(
        // Put cursor past the end of the input text
        chat_page_chunks[1].x + app.ui.chat_input.width() as u16 + 1,
        // Move one line down, from the border to the input line
        chat_page_chunks[1].y + 1,
    );
    let chat_input_paragraph = Paragraph::new(chat_input_text)
        .block(
            Block::default()
                .title(Span::styled("Input", Style::default()))
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    frame.render_widget(chat_input_paragraph, chat_page_chunks[1]);
}

pub fn draw_connection_page<B: Backend>(frame: &mut Frame<B>, size: Rect, app: &mut app::App) {
    let connection_page_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(size);

    // Connection Log
    // Regenerate Swarm Button
    let connection_log_style = if app.ui.connection_page_focus == ConnectionPageFocus::ConnectionLog
    {
        Style::default().add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
    };
    let connection_log_items = app
        .connection
        .log
        .iter()
        .map(|log_entry| ListItem::new(Text::styled(log_entry, Style::default().fg(Color::Gray))))
        .collect::<Vec<ListItem>>();

    let connection_log_list = List::new(connection_log_items)
        .block(
            Block::default()
                .title(Span::styled("Connection Log", connection_log_style))
                .borders(Borders::ALL)
                .border_type(BorderType::Plain),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    app.ui.connection_log_allocation = Some(connection_page_chunks[0]);

    frame.render_stateful_widget(
        connection_log_list,
        connection_page_chunks[0],
        &mut app.ui.connection_log_liststate,
    );

    // Regenerate Swarm Button
    let regenerate_button_style =
        if app.ui.connection_page_focus == ConnectionPageFocus::RegenerateSwarm {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().bg(Color::DarkGray)
        };
    let regenerate_button = Block::default()
        .title(Span::styled(
            "Regenerate Connection",
            regenerate_button_style,
        ))
        .borders(Borders::NONE);
    frame.render_widget(regenerate_button, connection_page_chunks[1]);

    // Address Input Field
    let addr_input_span = Span::styled(app.ui.addr_input.as_str(), Style::default());
    let addr_input_field_style =
        if app.ui.connection_page_focus == ConnectionPageFocus::AddrInputField {
            // Chat Input paragraph
            frame.set_cursor(
                // Put cursor past the end of the input text
                connection_page_chunks[2].x + app.ui.addr_input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                connection_page_chunks[2].y + 1,
            );
            Style::default().add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
        };
    let addr_input_field = Paragraph::new(addr_input_span).block(
        Block::default()
            .title(Span::styled("Connect to address", addr_input_field_style))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain),
    );
    frame.render_widget(addr_input_field, connection_page_chunks[2]);
}
