use unicode_width::UnicodeWidthStr;
use crossterm::{
	event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
	backend::{Backend, CrosstermBackend},
	layout::{Constraint, Direction, Layout},
	style::{Color, Style},
	text::{Span, Spans},
	widgets::{Block, Borders, BorderType, List, ListItem, Paragraph},
	Frame, Terminal,
};
mod secure;
mod strings;

#[derive(Clone)]
enum InputMode {
	Normal,
	Editing,
}

#[derive(PartialEq, Clone)]
enum AppState {
	Auth,
	Main
}

#[derive(PartialEq, Clone)]
enum ChatState {
	Disconnected,
	Connected(String),
	Error(String)
}

#[derive(Clone)]
struct App {
	input: String,
	inputs: [String; 3],
	auth_key: String,
	input_mode: InputMode,
	input_focus: u8,
	max_input_focus: u8,
	app_state: AppState,
	chat_state: ChatState,
	messages: Vec<String>,
}

impl App {
	fn initial() -> App {
		App {
			input: String::new(),
			inputs: ["".to_string(), "".to_string(), "".to_string()],
			auth_key: String::new(),
			input_mode: InputMode::Normal,
			input_focus: 0,
			max_input_focus: 1,
			app_state: AppState::Auth,
			chat_state: ChatState::Disconnected,
			messages: Vec::new(),
		}
	}
}

fn main() -> Result<(), Box<dyn Error>> {
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let app = App::initial();
	let res = run_app(&mut terminal, app);
	disable_raw_mode()?;
	execute!(
		terminal.backend_mut(),
		LeaveAlternateScreen,
		DisableMouseCapture
	)?;
	terminal.show_cursor()?;
	if let Err(err) = res { println!("Runtime error occured:\n{:?}", err) }
	Ok(())
}

fn change_state(to: AppState, app: &mut App) {
	match to {
		AppState::Main => {
			app.auth_key = app.inputs[0].clone();
			app.max_input_focus = 3
		}
		AppState::Auth => {
			app.max_input_focus = 1
		}
	}
	app.input_focus = 0;
	app.inputs = ["".to_string(), "".to_string(), "".to_string()];
	app.app_state = to;
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
	loop {
		match app.app_state {
			AppState::Auth => {
				terminal.draw(|f| auth_ui(f, &app))?;
			},
			AppState::Main => {
				terminal.draw(|f| main_ui(f, &app))?;
			}
		}
		if let Event::Key(key) = event::read()? {
			match key.modifiers {
				KeyModifiers::CONTROL => {
					if key.code == KeyCode::Char('c') {
						return Ok(())
					}
				},
				_ => match key.code {
					KeyCode::F(9) => return Ok(()),
					KeyCode::F(8) if app.app_state == AppState::Main => {
						app.auth_key = "".to_string();
						unimplemented!()
					}
					KeyCode::Up => {
						if app.input_focus <= 0 { app.input_focus = app.max_input_focus }
						else { app.input_focus -= 1 }
					},
					KeyCode::Down => {
						if app.input_focus >= app.max_input_focus { app.input_focus = 0 }
						else { app.input_focus += 1 }
					},
					a @ _ => if app.input_focus != 0 {
						match a {
							KeyCode::Char(c) => app.inputs[(app.input_focus - 1) as usize].push(c),
							KeyCode::Backspace => { (app.inputs[(app.input_focus - 1) as usize].pop()); },
							KeyCode::Enter => {
								if app.app_state == AppState::Auth && app.input_focus == 1 {
									change_state(AppState::Main, &mut app)
								} else if app.app_state == AppState::Main {
									unimplemented!()
								};
							},
							_ => ()
						}
					}
				}
			}
		}
	}
}

fn auth_ui<B: Backend>(f: &mut Frame<B>, app: &App) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.margin(1)
		.constraints(
			[
				Constraint::Length(6),
				Constraint::Length(3),
				Constraint::Min(1),
				Constraint::Length(4)
			]
			.as_ref(),
		)
		.split(f.size());
	let header = Paragraph::new(strings::LOGO.to_owned() + &format!("v{} ({})", env!("CARGO_PKG_VERSION"), secure::SERVER_NAME))
		.style(match app.input_focus {
			0 => Style::default().fg(Color::Cyan),
			_ => Style::default()
		});
	f.render_widget(header, chunks[0]);
	let input = Paragraph::new(app.inputs[0].as_ref())
		.style(match app.input_focus {
			1 => Style::default().fg(Color::Cyan),
			_ => Style::default()
		})
		.block(Block::default().borders(Borders::ALL)
			.title(match app.input_focus {
				1 => " Auth key (ENTER to submit) ",
				_ => " Auth key "
			})
			.border_type(match app.input_focus {
				1 => BorderType::Thick,
				_ => BorderType::Double
			})
		);
	f.render_widget(input.clone(), chunks[1]);
	let instructions = Paragraph::new(strings::USAGE_INSTRUCTIONS);
	f.render_widget(instructions, chunks[3]);
	if app.input_focus == 1 {
		f.set_cursor(
			chunks[1].x + app.inputs[0].width() as u16 + 1,
			chunks[1].y + 1,
		)
	}
}

fn main_ui<B: Backend>(f: &mut Frame<B>, app: &App) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(
			[
				Constraint::Length(2),
				Constraint::Length(3),
				Constraint::Length(3),
				Constraint::Min(1),
				Constraint::Length(3),
			]
			.as_ref(),
		)
		.split(f.size());
	let cs = match &app.chat_state {
		ChatState::Disconnected => "Untied".to_string(),
		ChatState::Connected(a) => format!("Tied with {}", a),
		ChatState::Error(a) => format!("Error: {}", a)
	};
	let hint = match app.input_focus {
		0 => " / ENTER to Log out",
		_ => ""
	};
	let header = Paragraph::new(format!("Husky v{} / {} / {}{}", env!("CARGO_PKG_VERSION"), app.auth_key/*app.auth_key.split(":").collect::<Vec<&str>>()[0]*/, cs, hint))
		.style(match app.input_focus {
			0 => Style::default().fg(Color::Cyan),
			_ => Style::default()
		});
	f.render_widget(header, chunks[0]);
	let interlocutor_input = Paragraph::new(app.inputs[0].as_ref())
		.style(match app.input_focus {
			1 => Style::default().fg(Color::Cyan),
			_ => Style::default()
		})
		.block(Block::default().borders(Borders::ALL)
			.title(match app.input_focus {
				1 => " Username (ENTER to start tie) ",
				_ => " Username "
			})
			.border_type(match app.input_focus {
				1 => BorderType::Thick,
				_ => BorderType::Double
			})
		);
	f.render_widget(interlocutor_input, chunks[1]);
	let encryption_key_input = Paragraph::new(app.inputs[1].as_ref())
		.style(match app.input_focus {
			2 => Style::default().fg(Color::Cyan),
			_ => Style::default()
		})
		.block(Block::default().borders(Borders::ALL)
			.title(" Encryption key ")
			.border_type(match app.input_focus {
				2 => BorderType::Thick,
				_ => BorderType::Double
			})
		);
	f.render_widget(encryption_key_input, chunks[2]);
	let messages: Vec<ListItem> = app
		.messages
		.iter()
		.enumerate()
		.map(|(i, m)| {
			let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
			ListItem::new(content)
		})
		.collect();
	let messages =
		List::new(messages).block(Block::default().style(Style::default().fg(Color::Gray)).borders(Borders::ALL).title(" Messages "));
	f.render_widget(messages, chunks[3]);
	let new_message_input = Paragraph::new(app.inputs[2].as_ref())
		.style(match app.input_focus {
			3 => Style::default().fg(Color::Cyan),
			_ => Style::default()
		})
		.block(Block::default().borders(Borders::ALL)
			.title(match app.input_focus {
				3 => " Message (ENTER to send) ",
				_ => " Message "
			})
			.border_type(match app.input_focus {
				3 => BorderType::Thick,
				_ => BorderType::Double
			})
		);
	f.render_widget(new_message_input, chunks[4]);
	if app.input_focus != 0 {
		f.set_cursor(
			chunks[(app.input_focus) as usize].x + app.inputs[(app.input_focus - 1) as usize].width() as u16 + 1,
			chunks[(if app.input_focus == 3 { 4 } else { app.input_focus }) as usize].y + 1,
		)
	}
}