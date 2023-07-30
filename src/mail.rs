use std::io::Write as _;
use std::process::{Command, Stdio};

use anyhow::Context as _;

#[allow(clippy::module_name_repetitions /* more descriptive */)]
pub fn send_mail(to: &str, message: &str) -> anyhow::Result<()> {
	let mut process = Command::new("/usr/sbin/sendmail")
		.args(["-i", "--", to])
		.stdin(Stdio::piped())
		.spawn()
		.context("spawning sendmail")?;

	let stdin = process.stdin.as_mut().unwrap();

	stdin
		.write_all(message.as_bytes())
		.context("writing to sendmail")?;

	let status = process.wait().context("waiting for sendmail")?;
	anyhow::ensure!(status.success(), "sendmail exited with non-zero status");

	Ok(())
}
