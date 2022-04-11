/***************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2021

[COMMIT-MODIFIED FILE]
***************************/

use base64::decode;
use std::str;

/// Base64-encoded server key to use on server-side to validate client's authentity
const ENCODED_SERVER_KEY: &str = "***";
/// Base64-encoded server root url where `preconnect.php` file is located (w/o scheme)
const ENCODED_SERVER_ROOT_URL: &str = "***";
/// Base64-encoded server port to connect via websockets
const ENCODED_SERVER_PORT: &str = "***";
/// Base64-encoded server name to display on app auth screen
const ENCODED_SERVER_NAME: &str = "***";

#[derive(Clone)]
pub struct Server {
	pub key: String,
	pub root_url: String,
	pub port: String,
	pub name: String,
}
impl Server {
	fn decode_server_key() -> String {
		let s = decode(ENCODED_SERVER_KEY).unwrap();
		str::from_utf8(&s).unwrap().to_owned()
	}
	fn decode_server_root_url() -> String {
		let s = decode(ENCODED_SERVER_ROOT_URL).unwrap();
		str::from_utf8(&s).unwrap().to_owned()
	}
	fn decode_server_port() -> String {
		let s = decode(ENCODED_SERVER_PORT).unwrap();
		str::from_utf8(&s).unwrap().to_owned()
	}
	fn decode_server_name() -> String {
		let s = decode(ENCODED_SERVER_NAME).unwrap();
		str::from_utf8(&s).unwrap().to_owned()
	}
}
impl Default for Server {
	fn default() -> Server {
		Server {
			key: Server::decode_server_key(),
			root_url: Server::decode_server_root_url(),
			port: Server::decode_server_port(),
			name: Server::decode_server_name(),
		}
	}
}
