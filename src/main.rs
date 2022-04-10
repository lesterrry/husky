/***************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2021

***************************/

use crate::strings::flags::*;
use crate::strings::ui::*;
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
	Untied,
	Tied(String),
}

#[derive(PartialEq, Clone)]
enum JobState {
	InProgress,
	Ok,
	Err,
}

/// The job data is stored here. Job is a state when app is busy with something
#[derive(PartialEq, Clone)]
struct Job {
	title: String,
	progress: u16,
	state: JobState,
	log: Vec<String>,
	data: Vec<String>,
}

impl Job {
	fn default(with_title: String) -> Job {
		Job {
			title: with_title,
			progress: 0,
			state: JobState::InProgress,
			log: Vec::new(),
			data: Vec::new(),
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
			state: ChatState::Untied,
			messages: Vec::new(),
			typing_state_iteration: 0,
		}
	}
}
impl Chat {
	fn with_subject(with_subject: String) -> Chat {
		Chat {
			state: ChatState::Tied(with_subject),
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
	/// Change state of App's job (if current state is `Job` or `force` is `true`, otherwise do nothing)
	fn job_state_set(&mut self, job_state: JobState, force_err: bool) {
		// FIXME:
		// This is imo the only 'real' *unsafe* part of the story
		match &self.state {
			AppState::Job(job) => {
				let mut job = job.clone();
				job.state = job_state;
				self.state = AppState::Job(job)
			}
			_ if force_err => {
				self.state = AppState::Job(Job {
					title: FATAL_RUNTIME_ERROR.to_string(),
					progress: 0,
					state: job_state,
					log: Vec::new(),
					data: Vec::new(),
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
unsafe fn print_type_of<T>(_: &T) {
	APP.job_log_add(&format!("{}", std::any::type_name::<T>()))
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
			println!("{}\n{:?}", FATAL_RUNTIME_ERROR, err)
		}
		process::exit(0);
	}
}

/// App's lifecycle loop
async unsafe fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
	async unsafe fn perform() {
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
							} else if let AppState::Chat(chat) = &APP.state {
								match &APP.input_focus {
									0 => {
										if let ChatState::Untied = chat.state {
											set_state(AppState::Auth)
										} else {
											untie().await
										}
									}
									1 => APP.requested_job = 2,
									_ => unimplemented!(),
								}
							} else if let AppState::Job(job) = &APP.state {
								match &job.state {
									JobState::InProgress => (),
									JobState::Ok => {
										//set_state(*ok.clone());
										// FIXME:
										// Oh I know I know...
										// W/o these we sometimes get corrupt mem
										thread::sleep(time::Duration::from_millis(200));
										continue;
									}
									JobState::Err => {
										//set_state(*err.clone());
										thread::sleep(time::Duration::from_millis(200));
										continue;
									}
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
				continue;
			}
			2 => {
				APP.requested_job = 0;
				tokio::spawn(start_tie_job());
				continue;
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

/// Switch App's state to a corresponding one and reset all associated variables
unsafe fn set_state(to: AppState) {
	match &to {
		AppState::Chat(_) => APP.max_input_focus = 3,
		AppState::Auth => {
			APP.sending_queue_add(TX_DROPME_FLAG.to_string());
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

/// Daemon for acting on every incoming message
async unsafe fn read_ws(
	with: futures_util::stream::SplitStream<
		tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
	>,
) {
	with.for_each(|message| async {
		match message {
			Ok(ok) => match ok {
				Message::Text(txt) => {
					let chars = txt.chars();
					let flag = chars.clone().collect::<Vec<char>>()[0] as char;
					let _body: String = chars.skip(1).collect();
					match flag {
						RX_AUTH_OK_FLAG => {
							if let AppState::Job(job) = &APP.state {
								if job.title == AUTH_JOB {
									APP.job_log_add(JOB_SUCCESS);
									APP.job_progress_set(100);
									APP.job_state_set(JobState::Ok, false);
								} else {
									// TODO:
									// Panic?
								}
							}
						}
						RX_AUTH_FAULT_FLAG => {
							if let AppState::Job(job) = &APP.state {
								if job.title == AUTH_JOB {
									APP.job_log_add(AUTH_JOB_CONNECT_AUTH_FAULT);
									APP.job_state_set(JobState::Err, false)
								} else {
									// TODO:
									// Panic?
								}
							}
						}
						RX_TIE_OK_FLAG => {
							if let AppState::Job(job) = &APP.state {
								if job.title == TIE_JOB {
									let subject = job.data[0].to_string();
									APP.job_log_add(JOB_SUCCESS);
									APP.job_progress_set(100);
									APP.job_state_set(JobState::Ok, false);
								} else {
									// TODO:
									// Panic?
								}
							}
						}
						RX_TIE_OK_WAIT_FLAG => {
							if let AppState::Job(job) = &APP.state {
								if job.title == TIE_JOB {
									APP.job_log_add(TIE_JOB_AWAITING);
									APP.job_progress_set(50);
								} else {
									// TODO:
									// Panic?
								}
							}
						}
						RX_TIE_FAULT_NOUSER_FLAG => {
							if let AppState::Job(job) = &APP.state {
								if job.title == TIE_JOB {
									APP.job_log_add(TIE_JOB_FAULT_NOUSER);
									APP.job_state_set(JobState::Err, false)
								} else {
									// TODO:
									// Panic?
								}
							}
						}
						RXTX_UNTIE_FLAG => {
							if let AppState::Chat(chat) = &APP.state {
								if let ChatState::Tied(_) = chat.state {
									APP.job_state_set(JobState::Err, true);
									APP.job_log_add(TIE_BROKEN);
								} else {
									// TODO:
									// Panic?
								}
							}
						}
						RX_UNKNOWN_FLAG => {
							panic!("{}", RX_UNKNOWN_ERROR);
						}
						_ => APP.job_log_add(&txt),
					}
				}
				_ => (),
			},
			Err(_) => {
				APP.job_state_set(JobState::Err, false);
				APP.job_log_add(MESSAGE_CORRUPTED_ERROR)
			}
		}
	})
	.await;
	APP.job_state_set(JobState::Err, true);
	APP.job_log_add(CONNECTION_DROPPED_ERROR)
}

/// Daemon for sending messages from queue
async unsafe fn write_ws(
	mut with: futures_util::stream::SplitSink<
		WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
		Message,
	>,
) {
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
			if with.send(Message::Text(i.to_string())).await.is_err() {
				APP.job_log_add(AUTH_JOB_CONNECT_FAULT);
				APP.job_state_set(JobState::Err, false);
			}
			if i == &TX_DROPME_FLAG.to_string() {
				APP.sending_queue = Vec::new();
				APP.sending_queue_sent = 0;
				return;
			}
			APP.sending_queue_sent += 1;
		}
	}
}

/// Make new connection and return a socket
async unsafe fn ws_connect() -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, ()> {
	let url =
		url::Url::parse(&format!("ws://{}:{}", APP.server.root_url, APP.server.port)).unwrap();
	let connection = connect_async(url).await;
	if connection.is_err() {
		return Err(());
	}
	let (ws_stream, _) = connection.unwrap();
	Ok(ws_stream)
}

/// Stop tie if exists
async unsafe fn untie() {
	APP.sending_queue_add(RXTX_UNTIE_FLAG.to_string());
	set_state(AppState::Chat(Chat::default()));
}

/// Change App's state to `Job` and begin authorization
async unsafe fn start_auth_job() {
	APP.user_key = Some(UserKey::default(APP.inputs[0].clone()));
	set_state(AppState::Job(Job::default(AUTH_JOB.to_string())));
	APP.job_log_add(JOB_STARTING);
	APP.job_log_add(AUTH_JOB_PRECONNECT);
	let res = reqwest::get(format!(
		"http://{}/{}",
		APP.server.root_url, "preconnect.php"
	))
	.await;
	APP.job_progress_set(25);
	// I'm EXTREMELY sorry but I do slow things down purposefully just to enjoy the cool interfaces
	thread::sleep(time::Duration::from_millis(200));
	if res.is_ok() {
		APP.job_log_add(JOB_SUCCESS);
		let txt = res.unwrap().text().await;
		if txt.is_ok() {
			if txt.as_ref().unwrap() == "Ok" {
				APP.job_log_add(AUTH_JOB_CONNECT);
				APP.job_progress_set(50);
				let connection = ws_connect().await;
				APP.job_progress_set(70);
				match connection {
					Err(_) => {
						APP.job_log_add(AUTH_JOB_CONNECT_FAULT);
						APP.job_state_set(JobState::Err, false);
						return;
					}
					Ok(ok) => {
						APP.job_log_add(JOB_SUCCESS);
						let (write, read) = ok.split();
						let r = tokio::spawn(read_ws(read));
						let w = tokio::spawn(write_ws(write));
						APP.socket_handles = Some((w, r));
						APP.job_progress_set(90);
						APP.job_log_add(AUTH_JOB_CONNECT_AUTH);
						// FIXME:
						// Now this one was added as a stability precaution...
						thread::sleep(time::Duration::from_millis(100));
						APP.sending_queue_add(format!(
							"{}{}/{}",
							TX_AUTH_FLAG,
							APP.server.key,
							APP.user_key.clone().unwrap().full
						));
					}
				};
			} else {
				APP.job_log_add(AUTH_JOB_PRECONNECT_FAULT_DISAPPROVED);
				APP.job_state_set(JobState::Err, false);
				return;
			}
		} else {
			APP.job_log_add(AUTH_JOB_PRECONNECT_FAULT_PARSE);
			APP.job_state_set(JobState::Err, false);
			return;
		}
	} else {
		APP.job_log_add(AUTH_JOB_PRECONNECT_FAULT_GET);
		APP.job_state_set(JobState::Err, false);
		return;
	}
}

/// Change App's state to `Job` and begin tying
async unsafe fn start_tie_job() {
	let subject = APP.inputs[0].clone();
	let mut job = Job::default(TIE_JOB.to_string());
	job.data = vec![subject.clone()];
	set_state(AppState::Job(job));
	APP.job_log_add(&format!("{} {}...", TIE_JOB_WITH, subject));
	thread::sleep(time::Duration::from_millis(500));
	APP.sending_queue_add(format!("{}{}", TX_TIE_INIT_FLAG, subject));
}

/// Renders app's `Job` state UI
unsafe fn job_ui<B: Backend>(f: &mut Frame<B>) {
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
					JobState::Ok => Color::Green,
					JobState::Err => Color::Red,
				}));
			f.render_widget(main_window, chunks[0]);
			{
				let chunks = Layout::default()
					.direction(Direction::Vertical)
					.vertical_margin(2)
					.horizontal_margin(4)
					.constraints(
						[
							Constraint::Length(3),
							Constraint::Min(1),
							Constraint::Length(1),
							Constraint::Length(1),
						]
						.as_ref(),
					)
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
					.map(|(_, m)| {
						let content = vec![Spans::from(Span::raw(m))];
						ListItem::new(content)
					})
					.collect();
				let log = List::new(log_messages).block(
					Block::default()
						.borders(Borders::ALL)
						.style(Style::default().fg(Color::White))
						.title(LOG_BLOCK)
						.title_alignment(Alignment::Center),
				);
				f.render_widget(log, chunks[1]);
				let prompt = Paragraph::new(PROMPT)
					.style(Style::default())
					.alignment(Alignment::Center);
				match job.state {
					JobState::InProgress => (),
					_ => f.render_widget(prompt, chunks[3]),
				}
			}
		}
		_ => {
			// TODO:
			// More descriptive errors (maybe)
			panic!("{}", FATAL_RUNTIME_ERROR);
		}
	}
}

/// Renders app's `Auth` state UI
unsafe fn auth_ui<B: Backend>(f: &mut Frame<B>) {
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
	let header = Paragraph::new(format!(
		"{}v{} ({})",
		LOGO,
		env!("CARGO_PKG_VERSION"),
		APP.server.name
	))
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
					AUTH_KEY_BLOCK_ACTIVE
				} else {
					AUTH_KEY_BLOCK_INACTIVE
				})
				.border_type(if APP.input_focus == 1 {
					BorderType::Thick
				} else {
					BorderType::Double
				}),
		);
	f.render_widget(input.clone(), chunks[1]);
	let instructions = Paragraph::new(USAGE_INSTRUCTIONS);
	f.render_widget(instructions, chunks[3]);
	if APP.input_focus == 1 {
		f.set_cursor(
			chunks[1].x + APP.inputs[0].width() as u16 + 1,
			chunks[1].y + 1,
		)
	}
}

/// Renders app's `Chat` state UI
unsafe fn chat_ui<B: Backend>(f: &mut Frame<B>) {
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
				ChatState::Untied => CHAT_STATE_UNTIED.to_string(),
				ChatState::Tied(a) => format!("{} {}", CHAT_STATE_TIED_WITH, a),
			};
			let hint = if APP.input_focus == 0 {
				if let ChatState::Untied = chat.state {
					CHAT_STATE_LOGOUT_PROMPT
				} else {
					CHAT_STATE_UNTIE_PROMPT
				}
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
			let subject_input = Paragraph::new(APP.inputs[0].as_ref())
				.style(match APP.input_focus {
					1 => Style::default().fg(Color::Cyan),
					_ => Style::default(),
				})
				.block(
					Block::default()
						.borders(Borders::ALL)
						.title(match APP.input_focus {
							1 => USERNAME_BLOCK_ACTIVE,
							_ => USERNAME_BLOCK_INACTIVE,
						})
						.border_type(match APP.input_focus {
							1 => BorderType::Thick,
							_ => BorderType::Double,
						}),
				);
			f.render_widget(subject_input, chunks[1]);
			let encryption_key_input = Paragraph::new(APP.inputs[1].as_ref())
				.style(match APP.input_focus {
					2 => Style::default().fg(Color::Cyan),
					_ => Style::default(),
				})
				.block(
					Block::default()
						.borders(Borders::ALL)
						.title(ENCRYPTION_KEY_BLOCK)
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
							3 => NEW_MESSAGE_BLOCK_ACTIVE,
							_ => NEW_MESSAGE_BLOCK_INACTIVE,
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
			panic!("{}", FATAL_RUNTIME_ERROR);
		}
	}
}
