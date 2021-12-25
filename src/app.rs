use std::io::Stdout;

use crate::connection::Connection;
use crate::input::{self, InputTask};
use crate::ui::{self, Ui};

use anyhow::Context;
use crossterm::event::EventStream;
use futures::{select, FutureExt, StreamExt};
use libp2p::swarm::SwarmEvent;
use tui::backend::CrosstermBackend;
use tui::Terminal;

#[derive(Debug, Clone)]
pub struct Message {
    pub text: String,
}

impl Message {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

pub struct App {
    pub ui: Ui,
    pub history: Vec<Message>,
    pub connection: Connection,
}

// Starting in IdleState
impl App {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            ui: Ui::new(),
            history: vec![],
            connection: Connection::new().context("Connection::new() failed in App::new()")?,
        })
    }

    pub async fn run_app(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), anyhow::Error> {
        let mut reader = EventStream::new();
        loop {
            let mut event = reader.next().fuse();

            select! {
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            match input::handle_input_event(event, &mut self) {
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
                event = self.connection.swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        self.connection.push_log_entry(format!("Listening on {:?}", address).as_str());
                    },
                    SwarmEvent::Behaviour(event) => {
                        self.connection.push_log_entry(format!("{:?}", event).as_str());
                    },
                    _ => {}
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
