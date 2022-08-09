/***************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2022

***************************/

#[rustfmt::skip]
#[allow(dead_code)]
pub mod ui {
	pub const LOGO: &str = 
	"   __            __       
	  / /  __ _____ / /____ __
	 / _ \\/ // (_-</  '_/ // /
	/_//_/\\_,_/___/_/\\_\\\\_, / 
	                   /___/  ";

	pub const USAGE_INSTRUCTIONS: &str = 
	"USAGE INSTRUCTIONS:
	[ARROW UP] / [ARROW DOWN] – Switch between active input blocks
	[ENTER] – Submit input
	[F9] / [CTRL + C] – Exit";

	pub const FATAL_RUNTIME_ERROR: 
		&str = "WARNING: FATAL";
	pub const CONNECTION_DROPPED_ERROR: 
		&str = "Websocket connection dropped";
	pub const MESSAGE_CORRUPTED_ERROR: 
		&str = "Unable to read from connection stream";
	pub const RX_GENERAL_ERROR: 
		&str = "Recieved client error from socket";
	pub const USERNAME_BLOCK_INACTIVE: 
		&str = " Username ";
	pub const USERNAME_BLOCK_ACTIVE: 
		&str = " Username (ENTER to initiate tie) ";
	pub const USERNAME_BLOCK_FILL_TIED: 
		&str = "[TIED]";
	pub const AUTH_KEY_BLOCK_INACTIVE: 
		&str = " Auth key ";
	pub const AUTH_KEY_BLOCK_ACTIVE: 
		&str = " Auth key (ENTER to submit) ";
	pub const MESSAGES_BLOCK_TYPING: 
		[&str; 4] = ["Typing   ", "Typing.  ", "Typing.. ", "Typing..."];
	pub const NEW_MESSAGE_BLOCK_INACTIVE:	
		&str = " Message ";
	pub const NEW_MESSAGE_BLOCK_ACTIVE: 
		&str = " Message (ENTER to send) ";
	pub const ENCRYPTION_KEY_BLOCK_INACTIVE: 
		&str = " Encryption key ";
	pub const ENCRYPTION_KEY_BLOCK_ACTIVE_ENABLE: 
		&str = " Encryption key (ENTER to enable encryption)";
	pub const ENCRYPTION_KEY_BLOCK_ACTIVE_DISABLE: 
		&str = " Encryption key (ENTER to disable encryption)";
	pub const CHAT_STATE_UNTIED: 
		&str = "Untied";
	pub const CHAT_STATE_TIED_WITH: 
		&str = "Tied with";
	pub const CHAT_STATE_ERROR: 
		&str = "Error";
	pub const CHAT_STATE_LOGOUT_PROMPT: 
		&str = " / ENTER to Log out";
	pub const CHAT_STATE_UNTIE_PROMPT: 
		&str = " / ENTER to Untie";
	pub const ENCRYPTION_STATE_ENCRYPTED: 
		&str = "Encrypted";
	pub const ENCRYPTION_STATE_NOT_ENCRYPTED: 
		&str = "NOT ENCRYPTED";
	pub const CONTINUE_PROMPT: 
		&str = "[ ENTER to continue ]";
	pub const ABORT_PROMPT: 
		&str = "[ ENTER to abort ]";
	pub const LOG_BLOCK: 
		&str = " Progress log ";
	pub const TIE_BROKEN:
		&str = "Tie broken";
	pub const JOB_STARTING: 
		&str = "Starting...";
	pub const JOB_SUCCESS: 
		&str = "SUCCESS";
	pub const AUTH_JOB: 
		&str = "Authorizing...";
	pub const AUTH_JOB_PRECONNECT: 
		&str = "Reaching server...";
	pub const AUTH_JOB_PRECONNECT_FAULT_PARSE: 
		&str = "FAULT: Unable to parse server response";
	pub const AUTH_JOB_PRECONNECT_FAULT_GET: 
		&str = "FAULT: Unable to get response from server";
	pub const AUTH_JOB_PRECONNECT_FAULT_DISAPPROVED: 
		&str = "FAULT: Connection not approved. Try again later";
	pub const AUTH_JOB_CONNECT: 
		&str = "Connecting to socket...";
	pub const AUTH_JOB_CONNECT_AUTH: 
		&str = "Sending auth data...";
	pub const AUTH_JOB_CONNECT_AUTH_AWAITING: 
		&str = "Awaiting response...";
	pub const AUTH_JOB_CONNECT_AUTH_FAULT: 
		&str = "FAULT: Access denied";
	pub const AUTH_JOB_CONNECT_AUTH_FAULT_OVERAUTH: 
		&str = "FAULT: User already logged in";
	pub const AUTH_JOB_CONNECT_FAULT: 
		&str = "FAULT: Unable to communicate with socket";
	pub const TIE_JOB:
		&str = "Tying...";
	pub const TIE_JOB_WITH:
		&str = "Tying with";
	pub const TIE_JOB_AWAITING:
		&str = "Waiting for subject to connect...";
	pub const TIE_JOB_UNTYING:
		&str = "Breaking existing tie...";
	pub const TIE_JOB_FAULT_NOUSER:
		&str = "FAULT: This user does not exist";
	pub const TIE_JOB_FAULT_SELFTIE:
		&str = "FAULT: Attempt to tie with self";
	pub const TIE_JOB_FAULT_OVERTIE:
		&str = "FAULT: Existing tie not broken";
	pub const ASTERISK:
		&str = "*";	
}

#[rustfmt::skip]
#[allow(dead_code)]
pub mod flags {
	pub const TX_AUTH_FLAG: 
		char = 'A';
	pub const TX_DROPME_FLAG: 
		char = 'X';
	pub const TX_TIE_INIT_FLAG: 
		char = 'T';
	pub const RX_AUTH_OK_FLAG: 
		char = 'O';
	pub const RX_AUTH_FAULT_FLAG: 
		char = 'D';
	pub const RX_AUTH_FAULT_OVERAUTH_FLAG:
		char = 'I';
	pub const RX_TIE_OK_FLAG:
		char = 'S';
	pub const RX_TIE_OK_WAIT_FLAG:
		char = 'W';
	pub const RX_TIE_FAULT_NOUSER_FLAG:
		char = 'N';
	pub const RX_TIE_FAULT_SELFTIE_FLAG:
		char = 'M';
	pub const RX_TIE_FAULT_OVERTIE_FLAG:
		char = 'R';
	pub const RXTX_UNTIE_FLAG:
		char = 'C';
	pub const RXTX_OK_FLAG:
		char = 'Y';
	pub const RXTX_FAULT_FLAG: 
		char = 'E';
	pub const RXTX_MESSAGE_FLAG: 
		char = 'B';
}
