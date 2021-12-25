use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use libp2p::Multiaddr;

use crate::app::{App, Message};
use crate::connection::Connection;
use crate::ui::{ConnectionPageFocus, CycleFocus, PageFocus};
use crate::utils;

pub enum InputTask {
    Continue,
    Quit,
}

pub fn handle_input_event(event: Event, app: &mut App) -> Result<InputTask, anyhow::Error> {
    // Cycle through pages with tab
    match event {
        Event::Key(key_event) => match (key_event.code, key_event.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => {
                app.ui.page_focus = app.ui.page_focus.next();
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                // request closing the app
                return Ok(InputTask::Quit);
            }
            _ => (),
        },
        _ => (),
    }

    match app.ui.page_focus {
        PageFocus::Chat => handle_input_event_chat_page(event, app)?,
        PageFocus::Connection => handle_input_event_connection_page(event, app)?,
    }

    Ok(InputTask::Continue)
}

pub fn handle_input_event_chat_page(event: Event, app: &mut App) -> Result<(), anyhow::Error> {
    match event {
        Event::Key(key_event) => match (key_event.code, key_event.modifiers) {
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                app.ui.chat_input.pop();
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                app.history.push(Message::new(app.ui.chat_input.clone()));
                app.ui.chat_input.clear();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                app.ui.chat_input.clear();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                app.ui.chat_input.push(c);
            }
            _ => (),
        },
        _ => (),
    };

    Ok(())
}

pub fn handle_input_event_connection_page(
    event: Event,
    app: &mut App,
) -> Result<(), anyhow::Error> {
    // Cycle through the different fields
    match event {
        Event::Key(key_event) => match (key_event.code, key_event.modifiers) {
            (KeyCode::Down, KeyModifiers::NONE) => {
                app.ui.connection_page_focus = app.ui.connection_page_focus.next();
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                app.ui.connection_page_focus = app.ui.connection_page_focus.prev();
            }
            _ => (),
        },
        _ => (),
    };

    match app.ui.connection_page_focus {
        ConnectionPageFocus::ConnectionLog => match event {
            Event::Mouse(mouse_event) => {
                let mouse_coord = (mouse_event.column, mouse_event.row);

                match mouse_event.kind {
                    MouseEventKind::ScrollDown => {
                        if let Some(allocation) = app.ui.connection_log_allocation {
                            if utils::coord_in_rect(mouse_coord, allocation) {
                                app.connection_log_next();
                            }
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if let Some(allocation) = app.ui.connection_log_allocation {
                            if utils::coord_in_rect(mouse_coord, allocation) {
                                app.connection_log_previous();
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        },
        ConnectionPageFocus::RegenerateSwarm => {
            match event {
                Event::Key(key_event) => match (key_event.code, key_event.modifiers) {
                    (KeyCode::Enter, KeyModifiers::NONE) => match Connection::regenerate_swarm() {
                        Ok(swarm) => app.connection.swarm = swarm,
                        Err(e) => {
                            log::error!("regenerate_swarm() failed with Err {}", e);
                        }
                    },
                    _ => (),
                },
                _ => (),
            };
        }
        ConnectionPageFocus::AddrInputField => {
            match event {
                Event::Key(key_event) => match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                        app.ui.addr_input.push(c);
                    }
                    (KeyCode::Backspace, KeyModifiers::NONE) => {
                        app.ui.addr_input.pop();
                    }
                    (KeyCode::Enter, KeyModifiers::NONE) => {
                        match app.ui.addr_input.parse::<Multiaddr>() {
                            Ok(dialed) => {
                                app.connection.dial(dialed.clone()).unwrap_or_else(|e| {
                                    app.connection.push_log_entry(
                                        format!(
                                            "dialing to addr {:?} failed with Err {}",
                                            dialed, e
                                        )
                                        .as_str(),
                                    );
                                });
                            }
                            Err(e) => {
                                app.connection.push_log_entry(
                                    format!("parsing input as MultiAddr failed with Err {}", e)
                                        .as_str(),
                                );
                            }
                        }
                    }
                    _ => (),
                },
                _ => (),
            };
        }
    }
    Ok(())
}
