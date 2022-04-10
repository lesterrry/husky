/***************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2021

***************************/

use chrono::Local;
use crossterm::{
	event::{self, Event, KeyCode, KeyModifiers},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::{SinkExt, StreamExt};
use reqwest;
use std::{error::Error, io, panic, process, thread, time};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
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
}

#[derive(PartialEq, Clone)]
enum JobState {
	InProgress,
	Ok(Box<AppState>),
	Err(Box<AppState>),
}

/// The job data is stored here. Job is a state when app is busy with something
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
		let time = Local::now();
		let t_string = time.format("%H:%M:%S");
		self.log.push(format!("({}) {}", t_string, msg));
	}
}

/// The chat data is stored here
#[derive(PartialEq, Clone)]
struct Chat {
	state: ChatState,
	messages: Vec<String>,
	typing_state_iteration: u8,
}

impl Default for Chat {
	fn default() -> Chat {
		Chat {
			state: ChatState::Disconnected,
			messages: Vec::new(),
			typing_state_iteration: 0,
		}
	}
}

/// The user's auth key data is stored here
#[derive(PartialEq, Clone)]
struct UserKey {
	full: String,
	username: String,
}

impl UserKey {
	fn default(with_full: String) -> UserKey {
		UserKey {
			full: with_full.clone(),
			username: with_full.split(":").collect::<Vec<&str>>()[0].to_string(),
		}
	}
}

/// The main application data is stored here
struct App {
	server: secure::Server,
	user_key: Option<UserKey>,
	inputs: [String; 3],
	input_focus: u8,
	max_input_focus: u8,
	state: AppState,
	requested_exit: bool,
	// FIXME:
	// Oh this is the stupidest thing in this script
	// I just couldn't figure out a way to tame all the async stuff otherwise
	requested_job: u8,
	sending_queue: Vec<String>,
	sending_queue_sent: u8,
	socket_handles: Option<(tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)>,
}

impl App {
	/// Get initial App instance
	fn initial() -> App {
		App {
			server: secure::Server::default(),
			user_key: None,
			inputs: ["".to_string(), "".to_string(), "".to_string()],
			input_focus: 0,
			max_input_focus: 1,
			state: AppState::Auth,
			requested_exit: false,
			requested_job: 0,
			sending_queue: Vec::new(),
			sending_queue_sent: 0,
			socket_handles: None,
		}
	}
	// FIXME:
	// Uh I don't like this
	/// Get nullable const-friendly App instance
	const fn null() -> App {
		App {
			server: secure::Server {
				key: String::new(),
				root_url: String::new(),
				port: String::new(),
				name: String::new(),
			},
			user_key: None,
			inputs: [String::new(), String::new(), String::new()],
			input_focus: 0,
			max_input_focus: 1,
			state: AppState::Auth,
			requested_exit: false,
			requested_job: 0,
			sending_queue: Vec::new(),
			sending_queue_sent: 0,
			socket_handles: None,
		}
	}
	/// Add text to App's job (if current state is `Job`, otherwise do nothing)
	fn job_log_add(&mut self, msg: &str) {
		// FIXME:
		// This is imo the only 'real' *unsafe* part of the story
		match &self.state {
			AppState::Job(job) => {
				let mut job = job.clone();
				job.log_add(msg);
				self.state = AppState::Job(job)
			}
			_ => { /* TODO: Maybe panic? */ }
		}
	}
	/// Change progress of App's job (if current state is `Job`, otherwise do nothing)
	fn job_progress_set(&mut self, progress: u16) {
		// FIXME:
		// This is imo the only 'real' *unsafe* part of the story
		match &self.state {
			AppState::Job(job) => {
				let mut job = job.clone();
				job.progress = progress;
				self.state = AppState::Job(job)
			}
			_ => { /* TODO: Maybe panic? */ }
		}
	}
	/// Change state of App's job (if current state is `Job`, otherwise do nothing)
	fn job_state_set(&mut self, job_state: JobState, force: bool) {
		// FIXME:
		// This is imo the only 'real' *unsafe* part of the story
		match &self.state {
			AppState::Job(job) => {
				let mut job = job.clone();
				job.state = job_state;
				self.state = AppState::Job(job)
			}
			_ if force => {
				self.state = AppState::Job(Job {
					title: strings::FATAL_RUNTIME_ERROR.to_string(),
					progress: 0,
					state: JobState::Err(Box::new(AppState::Auth)),
					log: Vec::new(),
				})
			}
			_ => (),
		}
	}
	/// Safely add text to App's sending queue
	fn sending_queue_add(&mut self, msg: String) {
		if self.sending_queue_sent >= 10 {
			self.sending_queue = vec![msg]
		} else {
			self.sending_queue.push(msg)
		}
	}
}

// FIXME:
// I could not figure out a better workaround. I ought to though. It's unsafe. Scary. Brrrrr.
/// The main global App instance, initialized as nullable
static mut APP: App = App::null();

#[allow(dead_code)]
#[cfg(debug_assertions)]
fn print_type_of<T>(_: &T) {
	unsafe { APP.job_log_add(&format!("{}", std::any::type_name::<T>())) }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let orig_hook = panic::take_hook();
	panic::set_hook(Box::new(move |panic_info| {
		orig_hook(panic_info);
		disable_raw_mode().unwrap();
		process::exit(1);
	}));
	unsafe {
		enable_raw_mode()?;
		let mut stdout = io::stdout();
		execute!(stdout, EnterAlternateScreen)?;
		let backend = CrosstermBackend::new(stdout);
		let mut terminal = Terminal::new(backend)?;
		APP = App::initial();
		let result = run_app(&mut terminal).await;
		disable_raw_mode()?;
		execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
		terminal.show_cursor()?;
		if let Err(err) = result {
			println!("{}\n{:?}", strings::FATAL_RUNTIME_ERROR, err)
		}
		process::exit(0);
	}
}

/// App's lifecycle loop
async fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
	unsafe {
		async fn perform() {
			unsafe {
				loop {
					let event = event::read();
					if event.is_err() {
						return;
					}
					if let Event::Key(key) = event.unwrap() {
						match key.modifiers {
							KeyModifiers::CONTROL => {
								if key.code == KeyCode::Char('c') {
									APP.requested_exit = true;
									return;
								}
							}
							_ => match key.code {
								KeyCode::F(9) => {
									APP.requested_exit = true;
									return;
								}
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
								KeyCode::Enter => {
									if APP.state == AppState::Auth && APP.input_focus == 1 {
										APP.requested_job = 1
									} else if let AppState::Chat(_) = &APP.state {
										unimplemented!()
									} else if let AppState::Job(job) = &APP.state {
										match &job.state {
											JobState::InProgress => (),
											JobState::Ok(ok) => set_state(*ok.clone()),
											JobState::Err(err) => set_state(*err.clone()),
										}
									};
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
		tokio::spawn(perform());
		// TODO:
		// Is it ok that the interface is being updated all the time? I really don't know
		loop {
			thread::sleep(time::Duration::from_millis(50));
			if APP.requested_exit {
				return Ok(());
			}
			match &APP.requested_job {
				1 => {
					APP.requested_job = 0;
					tokio::spawn(start_auth_job());
				}
				_ => (),
			}
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
		}
	}
}

/// Switch App's state to a corresponding one and reset all associated variables
fn set_state(to: AppState) {
	unsafe {
		match &to {
			AppState::Chat(_) => APP.max_input_focus = 3,
			AppState::Auth => {
				APP.sending_queue_add(strings::TX_DROPME_FLAG.to_string());
				// if APP.socket_handles.is_some() {
				// 	APP.socket_handles.as_ref().unwrap().0.abort();
				// 	APP.socket_handles.as_ref().unwrap().1.abort();
				// }
				APP.max_input_focus = 1
			}
			_ => APP.max_input_focus = 1,
		}
		APP.input_focus = 0;
		APP.inputs = [String::new(), String::new(), String::new()];
		APP.state = to;
	}
}

/// Daemon for acting on every incoming message
async fn read_ws(
	with: futures_util::stream::SplitStream<
		tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
	>,
) {
	unsafe {
		with.for_each(|message| async {
			match message {
				Ok(ok) => match ok {
					Message::Text(txt) => {
						let chars = txt.chars();
						let flag = chars.clone().collect::<Vec<char>>()[0] as char;
						let _body: String = chars.skip(1).collect();
						match flag {
							strings::RX_AUTH_OK_FLAG => {
								if let AppState::Job(_) = APP.state {
									APP.job_log_add(strings::JOB_SUCCESS);
									APP.job_progress_set(100);
									APP.job_state_set(
										JobState::Ok(Box::new(AppState::Chat(Chat::default()))),
										false,
									);
								}
							}
							strings::RX_AUTH_FAULT_FLAG => {
								if let AppState::Job(_) = APP.state {
									APP.job_log_add(strings::AUTH_JOB_CONNECT_AUTH_FAULT);
									APP.job_state_set(
										JobState::Err(Box::new(AppState::Auth)),
										false,
									)
								}
							}
							_ => APP.job_log_add(&txt),
						}
					}
					_ => (),
				},
				Err(_) => {
					APP.job_state_set(JobState::Err(Box::new(AppState::Auth)), false);
					APP.job_log_add(strings::MESSAGE_CORRUPTED_ERROR)
				}
			}
		})
		.await;
		APP.job_state_set(JobState::Err(Box::new(AppState::Auth)), false);
		APP.job_log_add(strings::CONNECTION_DROPPED_ERROR)
	}
}

/// Daemon for sending messages from queue
async fn write_ws(
	mut with: futures_util::stream::SplitSink<
		WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
		Message,
	>,
) {
	unsafe {
		loop {
			// TODO:
			// Is sleeping a good idea?
			thread::sleep(time::Duration::from_millis(100));
			let mut sent: u8 = 0;
			let app_sent = APP.sending_queue_sent;
			if app_sent >= 10 {
				APP.sending_queue_sent = 0
			}
			for i in &APP.sending_queue {
				if sent < app_sent {
					sent += 1;
					continue;
				}
				with.send(Message::Text(i.to_string())).await;
				if i == &strings::TX_DROPME_FLAG.to_string() {
					APP.sending_queue = Vec::new();
					APP.sending_queue_sent = 0;
					return;
				}
				//APP.job_log_add(&format!("Will send. S={}, AS={}", sent, app_sent));
				APP.sending_queue_sent += 1;
			}
		}
	}
}

/// Make new connection and return a socket
async fn ws_connect() -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, ()> {
	unsafe {
		let url =
			url::Url::parse(&format!("ws://{}:{}", APP.server.root_url, APP.server.port)).unwrap();
		let connection = connect_async(url).await;
		if connection.is_err() {
			return Err(());
		}
		let (ws_stream, _) = connection.unwrap();
		Ok(ws_stream)
	}
}

/// Change App's state to `Job` and begin authorization
async fn start_auth_job() {
	unsafe {
		APP.user_key = Some(UserKey::default(APP.inputs[0].clone()));
		set_state(AppState::Job(Job::default(strings::AUTH_JOB.to_string())));
		APP.job_log_add(strings::JOB_STARTING);
		APP.job_log_add(strings::AUTH_JOB_PRECONNECT);
		let res = reqwest::get(format!(
			"http://{}/{}",
			APP.server.root_url, "preconnect.php"
		))
		.await;
		APP.job_progress_set(25);
		// I'm EXTREMELY sorry but I slow things down purposefully just to enjoy the cool interfaces
		thread::sleep(time::Duration::from_millis(500));
		if res.is_ok() {
			APP.job_log_add(strings::JOB_SUCCESS);
			let txt = res.unwrap().text().await;
			if txt.is_ok() {
				if txt.as_ref().unwrap() == "Ok" {
					APP.job_log_add(strings::AUTH_JOB_PRECONNECT_SUCCESS);
					APP.job_log_add(strings::AUTH_JOB_CONNECT);
					APP.job_progress_set(50);
					let connection = ws_connect().await;
					APP.job_progress_set(70);
					match connection {
						Err(_) => {
							APP.job_log_add(strings::AUTH_JOB_CONNECT_FAULT);
							APP.job_state_set(JobState::Err(Box::new(AppState::Auth)), false);
							return;
						}
						Ok(ok) => {
							APP.job_log_add(strings::JOB_SUCCESS);
							let (write, read) = ok.split();
							let r = tokio::spawn(read_ws(read));
							let w = tokio::spawn(write_ws(write));
							APP.socket_handles = Some((w, r));
							APP.job_progress_set(90);
							APP.job_log_add(strings::AUTH_JOB_CONNECT_AUTH);
							APP.sending_queue_add(format!(
								"{}{}/{}",
								strings::TX_AUTH_FLAG,
								APP.server.key,
								APP.user_key.clone().unwrap().full
							));
							APP.job_log_add(strings::AUTH_JOB_CONNECT_AUTH_AWAITING);
						}
					};
				} else {
					APP.job_log_add(&txt.unwrap());
					APP.job_state_set(JobState::Err(Box::new(AppState::Auth)), false);
					return;
				}
			} else {
				APP.job_log_add(strings::AUTH_JOB_PRECONNECT_FAULT_PARSE);
				APP.job_state_set(JobState::Err(Box::new(AppState::Auth)), false);
				return;
			}
		} else {
			APP.job_log_add(strings::AUTH_JOB_PRECONNECT_FAULT_GET);
			APP.job_state_set(JobState::Err(Box::new(AppState::Auth)), false);
			return;
		}
	}
}

/// Renders app's `Job` state UI
fn job_ui<B: Backend>(f: &mut Frame<B>) {
	unsafe {
		// TODO: Typing indicator
		// if APP.typing_state_iteration >= 3 {
		// 	APP.typing_state_iteration = 0
		// } else {
		// 	APP.typing_state_iteration += 1
		// }
		// ( .title(strings::MESSAGES_BLOCK_TYPING[APP.typing_state_iteration as usize]) )
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
					.style(Style::default().bg(match job.state {
						JobState::InProgress => Color::DarkGray,
						JobState::Ok(_) => Color::Green,
						JobState::Err(_) => Color::Red,
					}));
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
					// TODO:
					// It'd be cool to place `ListItem`s into `Job` instead of `String`s, so I could have the log formatted differently
					let log_messages: Vec<ListItem> = job
						.log
						.iter()
						.enumerate()
						.map(|(i, m)| {
							let content = vec![Spans::from(Span::raw(format!("{} {}", i + 1, m)))];
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
				// More descriptive errors (maybe)
				panic!("{}", strings::FATAL_RUNTIME_ERROR);
			}
		}
	}
}

/// Renders app's `Auth` state UI
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
				+ &format!("v{} ({})", env!("CARGO_PKG_VERSION"), APP.server.name),
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

/// Renders app's `Chat` state UI
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
				};
				let hint = if APP.input_focus == 0 {
					strings::CHAT_STATE_LOGOUT_PROMPT
				} else {
					""
				};
				let header = Paragraph::new(format!(
					"Husky v{} / {} / {}{}",
					env!("CARGO_PKG_VERSION"),
					APP.user_key.as_ref().unwrap().username,
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
				panic!("{}", strings::FATAL_RUNTIME_ERROR);
			}
		}
	}
}
