use std::io::Stdout;

use crate::connection::{self, Connection};
use crate::input::{self, InputTask};
use crate::ui::{self, Ui};

use anyhow::Context;
use crossterm::event::EventStream;
use futures::{select, StreamExt};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tui::backend::CrosstermBackend;
use tui::Terminal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    #[serde(skip)]
    pub source_peer_id: Option<PeerId>,
    pub nick: Option<String>,
    pub text: String,
}

impl ChatMessage {
    pub fn new(source_peer_id: Option<PeerId>, nick: Option<String>, text: String) -> Self {
        Self {
            source_peer_id,
            nick,
            text,
        }
    }
}

pub struct App {
    pub ui: Ui,
    pub history: Vec<ChatMessage>,
    pub connection: Connection,
}

// Starting in IdleState
impl App {
    pub async fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            ui: Ui::new(),
            history: vec![],
            connection: Connection::new()
                .await
                .context("Connection::new() failed in App::new()")?,
        })
    }

    pub async fn run(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), anyhow::Error> {
        let mut input_eventstream = EventStream::new().fuse();

        loop {
            select! {
                input_event = &mut input_eventstream.select_next_some() => {
                     match input_event {
                        Ok(input_event) => {
                            match input::handle_input_event(input_event, &mut self) {
                                Ok(input_task) => match input_task {
                                    InputTask::Continue => (),
                                    InputTask::Quit => break,
                                },
                                Err(e) => {
                                    log::error!("handle_input_event() failed with Err `{}`", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("input_event is Err `{}`", e);
                        }
                    }
                },
                connection_event = self.connection.swarm.select_next_some() => match connection::handle_connection_event(connection_event, &mut self) {
                    Ok(()) => {}
                    Err(e) => {
                        log::error!("handle_connection_event() failed with Err `{}`", e);
                    }
                }
            }

            ui::draw_ui(&mut self, terminal)?;
        }
        Ok(())
    }

    // Select the next item. This will not be reflected until the widget is drawn in the
    // `Terminal::draw` callback using `Frame::render_stateful_widget`.
    pub fn connection_log_next(&mut self) {
        let i = match self.ui.connection_log_liststate.selected() {
            Some(i) => {
                if i >= self.connection.log.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.ui.connection_log_liststate.select(Some(i));
    }

    // Select the previous item. This will not be reflected until the widget is drawn in the
    // `Terminal::draw` callback using `Frame::render_stateful_widget`.
    pub fn connection_log_previous(&mut self) {
        let i = match self.ui.connection_log_liststate.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.ui.connection_log_liststate.select(Some(i));
    }

    // Unselect the currently selected item if any. The implementation of `ListState` makes
    // sure that the stored offset is also reset.
    pub fn connection_log_unselect(&mut self) {
        self.ui.connection_log_liststate.select(None);
    }
}
