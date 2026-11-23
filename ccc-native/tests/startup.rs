//! Exercise native command parsing and process failure reporting.

use std::{
	io,
	process::{Command, Stdio},
};

#[test]
fn unknown_command_fails_before_starting_a_listener() {
	let output = Command::new(env!("CARGO_BIN_EXE_ccc-native"))
		.arg("unknown")
		.output()
		.expect("native process");
	assert_eq!(output.status.code(), Some(1));
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(stderr.contains("ccc.native.server_failed"));
	assert!(stderr.contains("usage: ccc-native [healthcheck]"));
}

#[test]
fn invalid_configuration_exits_one_even_when_stderr_is_closed() {
	let (reader, writer) = io::pipe().expect("stderr pipe");
	drop(reader);
	let status = Command::new(env!("CARGO_BIN_EXE_ccc-native"))
		.env("CCC_LISTEN_ADDR", "invalid")
		.stderr(Stdio::from(writer))
		.status()
		.expect("native process");
	assert_eq!(status.code(), Some(1));
}

#[test]
fn invalid_configuration_records_the_parser_error() {
	let output = Command::new(env!("CARGO_BIN_EXE_ccc-native"))
		.env("CCC_LISTEN_ADDR", "invalid")
		.output()
		.expect("native process");
	assert_eq!(output.status.code(), Some(1));
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(stderr.contains("ccc.native.server_failed"));
	assert!(stderr.contains("invalid socket address syntax"));
}
