use std::io::Stdout;

use crate::connection::{self, Connection};
use crate::input::{self, InputTask};
use crate::ui::{self, Ui};

use anyhow::Context;
use crossterm::event::EventStream;
use futures::{select, FutureExt, StreamExt};
use libp2p::PeerId;
use tui::backend::CrosstermBackend;
use tui::Terminal;

#[derive(Debug, Clone)]
pub struct Message {
    pub source_peer_id: Option<PeerId>,
    pub text: String,
}

impl Message {
    pub fn new(text: String, source_peer_id: Option<PeerId>) -> Self {
        Self { source_peer_id, text }
    }
}

pub struct App {
    pub ui: Ui,
    pub history: Vec<Message>,
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
        let mut input_eventstream = EventStream::new();
        let mut input_event = input_eventstream.next().fuse();

        loop {
            select! {
                maybe_input_event = input_event => {
                    // next input event
                    input_event = input_eventstream.next().fuse();

                     match maybe_input_event {
                        Some(Ok(input_event)) => {
                            match input::handle_input_event(input_event, &mut self) {
                                Ok(input_task) => match input_task {
                                    InputTask::Continue => (),
                                    InputTask::Quit => break,
                                },
                                Err(e) => {
                                    log::error!("handle_input_event() returned Err '{}'", e);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("Err {}", e);
                        }
                        None => break,
                    }
                },
                connection_event = self.connection.swarm.select_next_some() => match connection::handle_connection_event(connection_event, &mut self) {
                    Ok(()) => {}
                    Err(e) => {
                        log::error!("handle_connection_event() failed with Err {}", e);
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
