/***************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2021

***************************/

#[rustfmt::skip]
pub const LOGO: &str = 
"   __            __       
  / /  __ _____ / /____ __
 / _ \\/ // (_-</  '_/ // /
/_//_/\\_,_/___/_/\\_\\\\_, / 
                   /___/  ";

#[rustfmt::skip]
pub const USAGE_INSTRUCTIONS: &str = 
"USAGE INSTRUCTIONS:
[ARROW UP] / [ARROW DOWN] – Switch between active input blocks
[ENTER] – Submit input
[F9] / [CTRL + C] – Exit";

pub const FATAL_RUNTIME_ERROR: &str = "Runtime error occured:\n";
pub const USERNAME_BLOCK_INACTIVE: &str = " Username ";
pub const USERNAME_BLOCK_ACTIVE: &str = " Username (ENTER to initiate tie) ";
pub const AUTH_KEY_BLOCK_INACTIVE: &str = " Auth key ";
pub const AUTH_KEY_BLOCK_ACTIVE: &str = " Auth key (ENTER to submit) ";
pub const MESSAGES_BLOCK_TYPING: [&str; 4] = ["Typing   ", "Typing.  ", "Typing.. ", "Typing..."];
pub const NEW_MESSAGE_BLOCK_INACTIVE: &str = " Message ";
pub const NEW_MESSAGE_BLOCK_ACTIVE: &str = " Message (ENTER to send) ";
pub const ENCRYPTION_KEY_BLOCK: &str = " Encryption key ";
pub const CHAT_STATE_UNTIED: &str = "Untied";
pub const CHAT_STATE_TIED_WITH: &str = "Tied with";
pub const CHAT_STATE_ERROR: &str = "Error";
pub const CHAT_STATE_LOGOUT_PROMPT: &str = " / ENTER to Log out";
pub const LOG_BLOCK: &str = " Progress log ";
pub const JOB_STARTING: &str = "Starting...";
pub const AUTH_JOB: &str = "Authorizing...";
