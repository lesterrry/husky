/***************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2021

***************************/

use crossterm::{
	event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io, process};
use tui::{
	backend::{Backend, CrosstermBackend},
	layout::{Alignment, Constraint, Direction, Layout},
	style::{Color, Style},
	text::{Span, Spans},
	widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
	Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;
mod secure;
mod strings;

#[derive(PartialEq, Clone)]
enum AppState {
	Auth,
	Chat(Chat),
	Job(Job),
}

#[derive(PartialEq, Clone)]
enum ChatState {
	Disconnected,
	Connected(String),
	Error(String),
}

#[derive(PartialEq, Clone)]
enum JobState {
	InProgress,
	Ok,
	Err,
}

#[derive(PartialEq, Clone)]
struct Job {
	title: String,
	progress: u16,
	state: JobState,
	log: Vec<String>,
}

impl Job {
	fn default(with_title: String) -> Job {
		Job {
			title: with_title,
			progress: 0,
			state: JobState::InProgress,
			log: Vec::new(),
		}
	}
	fn log_add(&mut self, msg: &str) {
		self.log.push(msg.to_string())
	}
	fn log_clear(&mut self) {
		self.log = Vec::new()
	}
}

#[derive(PartialEq, Clone)]
struct Chat {
	auth_key: String,
	state: ChatState,
	messages: Vec<String>,
}

struct App {
	server: secure::Server,
	inputs: [String; 3],
	input_focus: u8,
	max_input_focus: u8,
	state: AppState,
}

impl App {
	fn initial() -> App {
		App {
			server: secure::Server::default(),
			inputs: ["".to_string(), "".to_string(), "".to_string()],
			input_focus: 0,
			max_input_focus: 1,
			state: AppState::Auth,
		}
	}
	// Uh I don't like this
	const fn null() -> App {
		App {
			server: secure::Server {
				server_key: String::new(),
				server_root_url: String::new(),
				server_name: String::new(),
			},
			inputs: [String::new(), String::new(), String::new()],
			input_focus: 0,
			max_input_focus: 1,
			state: AppState::Auth,
		}
	}
}

// TODO:
// I could not figure out a better workaround. I ought to though. It's unsafe. Scary. Brrrrr.
static mut APP: App = App::null();

fn start_auth_job() {
	fn log_add(msg: &str) {
		// TODO:
		// This is imo the only 'real' *unsafe* part of the story
		unsafe {
			match &APP.state {
				AppState::Job(job) => {
					let mut job = job.clone();
					job.log_add(msg);
					APP.state = AppState::Job(job)
				}
				_ => (),
			}
		}
	}
	change_state(AppState::Job(Job::default(strings::AUTH_JOB.to_string())));
	std::thread::spawn(move || log_add("Starting..."));
}

fn main() -> Result<(), Box<dyn Error>> {
	unsafe {
		enable_raw_mode()?;
		let mut stdout = io::stdout();
		execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
		let backend = CrosstermBackend::new(stdout);
		let mut terminal = Terminal::new(backend)?;
		APP = App::initial();
		let result = run_app(&mut terminal);
		disable_raw_mode()?;
		execute!(
			terminal.backend_mut(),
			LeaveAlternateScreen,
			DisableMouseCapture
		)?;
		terminal.show_cursor()?;
		if let Err(err) = result {
			println!("{}\n{:?}", strings::FATAL_RUNTIME_ERROR, err)
		}
		Ok(())
	}
}

fn change_state(to: AppState) {
	unsafe {
		match to.clone() {
			AppState::Chat(mut a) => {
				a.auth_key = APP.inputs[0].clone();
				APP.max_input_focus = 3
			}
			AppState::Auth => APP.max_input_focus = 1,
			_ => (),
		}
		APP.input_focus = 0;
		APP.inputs = ["".to_string(), "".to_string(), "".to_string()];
		APP.state = to;
	}
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
	unsafe {
		loop {
			match APP.state {
				AppState::Auth => {
					terminal.draw(|f| auth_ui(f))?;
				}
				AppState::Chat(_) => {
					terminal.draw(|f| chat_ui(f))?;
				}
				AppState::Job(_) => {
					terminal.draw(|f| job_ui(f))?;
				}
			}
			if let Event::Key(key) = event::read()? {
				match key.modifiers {
					KeyModifiers::CONTROL => {
						if key.code == KeyCode::Char('c') {
							return Ok(());
						}
					}
					_ => match key.code {
						KeyCode::F(9) => return Ok(()),
						KeyCode::Up => {
							if APP.input_focus <= 0 {
								APP.input_focus = APP.max_input_focus
							} else {
								APP.input_focus -= 1
							}
						}
						KeyCode::Down => {
							if APP.input_focus >= APP.max_input_focus {
								APP.input_focus = 0
							} else {
								APP.input_focus += 1
							}
						}
						a @ _ => {
							if APP.input_focus != 0 {
								match a {
									KeyCode::Char(c) => {
										APP.inputs[(APP.input_focus - 1) as usize].push(c)
									}
									KeyCode::Backspace => {
										(APP.inputs[(APP.input_focus - 1) as usize].pop());
									}
									KeyCode::Enter => {
										if APP.state == AppState::Auth && APP.input_focus == 1 {
											start_auth_job();
										// change_state(
										// 	AppState::Chat(Chat {
										// 		auth_key: app.inputs[0].clone(),
										// 		messages: Vec::new(),
										// 		state: ChatState::Disconnected,
										// 	}),
										// 	&mut app,
										// )
										} else if let AppState::Chat(_) = APP.state {
											unimplemented!()
										};
									}
									_ => (),
								}
							}
						}
					},
				}
			}
		}
	}
}

fn job_ui<B: Backend>(f: &mut Frame<B>) {
	unsafe {
		match &APP.state {
			AppState::Job(job) => {
				let chunks = Layout::default()
					.direction(Direction::Vertical)
					.vertical_margin(2)
					.horizontal_margin(12)
					.constraints([Constraint::Min(1)].as_ref())
					.split(f.size());
				let main_window = Block::default()
					.borders(Borders::NONE)
					.title(job.title.clone())
					.title_alignment(Alignment::Center)
					.style(Style::default().bg(Color::DarkGray));
				f.render_widget(main_window, chunks[0]);
				{
					let chunks = Layout::default()
						.direction(Direction::Vertical)
						.vertical_margin(2)
						.horizontal_margin(4)
						.constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
						.split(chunks[0]);
					let progress_bar = Gauge::default()
						.gauge_style(Style::default().fg(Color::White))
						.percent(job.progress)
						.label(Span::styled(
							format!("{}%", job.progress),
							Style::default().fg(Color::Black),
						))
						.block(
							Block::default()
								.borders(Borders::ALL)
								.style(Style::default().fg(Color::White)),
						);
					f.render_widget(progress_bar, chunks[0]);
					let log_messages: Vec<ListItem> = job
						.log
						.iter()
						.enumerate()
						.map(|(i, m)| {
							let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
							ListItem::new(content)
						})
						.collect();
					let log = List::new(log_messages).block(
						Block::default()
							.borders(Borders::ALL)
							.style(Style::default().fg(Color::White))
							.title(strings::LOG_BLOCK)
							.title_alignment(Alignment::Center),
					);
					f.render_widget(log, chunks[1]);
				}
			}
			_ => {
				// TODO:
				// This is very bad as it screws the terminal up. The good part is we'll hopefully never get here
				process::exit(1);
			}
		}
	}
}

fn auth_ui<B: Backend>(f: &mut Frame<B>) {
	unsafe {
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.margin(1)
			.constraints(
				[
					Constraint::Length(6),
					Constraint::Length(3),
					Constraint::Min(1),
					Constraint::Length(4),
				]
				.as_ref(),
			)
			.split(f.size());
		let header = Paragraph::new(
			strings::LOGO.to_owned()
				+ &format!(
					"v{} ({})",
					env!("CARGO_PKG_VERSION"),
					APP.server.server_name
				),
		)
		.style(if APP.input_focus == 0 {
			Style::default().fg(Color::Cyan)
		} else {
			Style::default()
		});
		f.render_widget(header, chunks[0]);
		let input = Paragraph::new(APP.inputs[0].as_ref())
			.style(if APP.input_focus == 1 {
				Style::default().fg(Color::Cyan)
			} else {
				Style::default()
			})
			.block(
				Block::default()
					.borders(Borders::ALL)
					.title(if APP.input_focus == 1 {
						strings::AUTH_KEY_BLOCK_ACTIVE
					} else {
						strings::AUTH_KEY_BLOCK_INACTIVE
					})
					.border_type(if APP.input_focus == 1 {
						BorderType::Thick
					} else {
						BorderType::Double
					}),
			);
		f.render_widget(input.clone(), chunks[1]);
		let instructions = Paragraph::new(strings::USAGE_INSTRUCTIONS);
		f.render_widget(instructions, chunks[3]);
		if APP.input_focus == 1 {
			f.set_cursor(
				chunks[1].x + APP.inputs[0].width() as u16 + 1,
				chunks[1].y + 1,
			)
		}
	}
}

fn chat_ui<B: Backend>(f: &mut Frame<B>) {
	unsafe {
		match &APP.state {
			AppState::Chat(chat) => {
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
				let cs = match &chat.state {
					ChatState::Disconnected => strings::CHAT_STATE_UNTIED.to_string(),
					ChatState::Connected(a) => format!("{} {}", strings::CHAT_STATE_TIED_WITH, a),
					ChatState::Error(a) => format!("{} {}", strings::CHAT_STATE_ERROR, a),
				};
				let hint = if APP.input_focus == 0 {
					strings::CHAT_STATE_LOGOUT_PROMPT
				} else {
					""
				};
				let header = Paragraph::new(format!(
					"Husky v{} / {} / {}{}",
					env!("CARGO_PKG_VERSION"),
					chat.auth_key, /*app.auth_key.split(":").collect::<Vec<&str>>()[0]*/
					cs,
					hint
				))
				.style(if APP.input_focus == 0 {
					Style::default().fg(Color::Cyan)
				} else {
					Style::default()
				});
				f.render_widget(header, chunks[0]);
				let interlocutor_input = Paragraph::new(APP.inputs[0].as_ref())
					.style(match APP.input_focus {
						1 => Style::default().fg(Color::Cyan),
						_ => Style::default(),
					})
					.block(
						Block::default()
							.borders(Borders::ALL)
							.title(match APP.input_focus {
								1 => strings::USERNAME_BLOCK_ACTIVE,
								_ => strings::USERNAME_BLOCK_INACTIVE,
							})
							.border_type(match APP.input_focus {
								1 => BorderType::Thick,
								_ => BorderType::Double,
							}),
					);
				f.render_widget(interlocutor_input, chunks[1]);
				let encryption_key_input = Paragraph::new(APP.inputs[1].as_ref())
					.style(match APP.input_focus {
						2 => Style::default().fg(Color::Cyan),
						_ => Style::default(),
					})
					.block(
						Block::default()
							.borders(Borders::ALL)
							.title(strings::ENCRYPTION_KEY_BLOCK)
							.border_type(match APP.input_focus {
								2 => BorderType::Thick,
								_ => BorderType::Double,
							}),
					);
				f.render_widget(encryption_key_input, chunks[2]);
				let messages: Vec<ListItem> = chat
					.messages
					.iter()
					.enumerate()
					.map(|(i, m)| {
						let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
						ListItem::new(content)
					})
					.collect();
				let messages = List::new(messages).block(
					Block::default()
						.style(Style::default().fg(Color::Gray))
						.borders(Borders::ALL),
				);
				f.render_widget(messages, chunks[3]);
				let new_message_input = Paragraph::new(APP.inputs[2].as_ref())
					.style(match APP.input_focus {
						3 => Style::default().fg(Color::Cyan),
						_ => Style::default(),
					})
					.block(
						Block::default()
							.borders(Borders::ALL)
							.title(match APP.input_focus {
								3 => strings::NEW_MESSAGE_BLOCK_ACTIVE,
								_ => strings::NEW_MESSAGE_BLOCK_INACTIVE,
							})
							.border_type(match APP.input_focus {
								3 => BorderType::Thick,
								_ => BorderType::Double,
							}),
					);
				f.render_widget(new_message_input, chunks[4]);
				if APP.input_focus != 0 {
					f.set_cursor(
						chunks[(APP.input_focus) as usize].x
							+ APP.inputs[(APP.input_focus - 1) as usize].width() as u16
							+ 1,
						chunks[(if APP.input_focus == 3 {
							4
						} else {
							APP.input_focus
						}) as usize]
							.y + 1,
					)
				}
			}
			_ => {
				// TODO:
				// This is very bad as it screws the terminal up. The good part is we'll hopefully never get here
				process::exit(1);
			}
		}
	}
}
