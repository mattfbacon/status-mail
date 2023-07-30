#![deny(
	absolute_paths_not_starting_with_crate,
	keyword_idents,
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_abi,
	missing_copy_implementations,
	non_ascii_idents,
	nonstandard_style,
	noop_method_call,
	pointer_structural_match,
	private_in_public,
	rust_2018_idioms,
	unused_qualifications
)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::fmt::Write;

use crate::mail::send_mail;

mod mail;
mod tests;

#[derive(
	Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
enum Status {
	#[default]
	Nominal,
	Warning,
	Critical,
}

impl Status {
	fn as_repr(self) -> &'static str {
		match self {
			Self::Nominal => "Nominal",
			Self::Warning => "Warning",
			Self::Critical => "Critical",
		}
	}
}

struct Report {
	status: Status,
	message: String,
}

trait Test {
	fn report(&self) -> anyhow::Result<Report>;
}

fn to_report(report: anyhow::Result<Report>) -> Report {
	report.unwrap_or_else(|error| Report {
		status: Status::Warning,
		message: error.to_string(),
	})
}

macro_rules! tests {
	($args:ident; $($name:ident = $create:expr,)*) => {
		const PERSISTENCE_PATH: &str = "persistence.json";

		#[derive(Default, serde::Serialize, serde::Deserialize)]
		struct Persistence {
			$($name: Status,)*
		}

		let mut persistence = match std::fs::read_to_string(PERSISTENCE_PATH) {
			Ok(s) => match serde_json::from_str(&s) {
				Ok(persistence) => persistence,
				Err(error) => {
					eprintln!("warning: error deserializing persistence: {error}");
					Persistence::default()
				}
			}
			Err(error) => {
				if error.kind() != std::io::ErrorKind::NotFound {
					eprintln!("warning: error reading persistence: {error}");
				}
				Persistence::default()
			}
		};

		let mut reports = Vec::new();
		let mut should_alert = false;

		$({
			let test = $create;
			let report = to_report(test.report());

			if report.status > persistence.$name {
				should_alert = true;
			}
			persistence.$name = report.status;

			reports.push(report);
		})*

		if let Err(error) = std::fs::write(PERSISTENCE_PATH, serde_json::to_vec(&persistence).unwrap()) {
			eprintln!("error writing to persistence: {error}");
		}

		if should_alert {
			reports.sort_by_key(|report| std::cmp::Reverse(report.status));

			let mut critical_count = 0;
			let mut warning_count = 0;
			let mut nominal_count = 0;

			for report in &reports {
				*match report.status {
					Status::Nominal => &mut nominal_count,
					Status::Warning => &mut warning_count,
					Status::Critical => &mut critical_count
				} += 1;
			}

			let mut message = String::with_capacity(40 + reports.len() * 20);
			write!(message, "\
				From: System Status <localdata@felle.nz>\n\
				To: matt@felle.nz\n\
				Subject: "
			).unwrap();

			if critical_count > 0 || warning_count > 0 {
				write!(message, "[!] ").unwrap();
			}
			write!(message, "System Status: ").unwrap();

			if critical_count > 0 {
				write!(message, "{critical_count} critical, ").unwrap();
			}

			if warning_count > 0 {
				write!(message, "{warning_count} warning, ").unwrap();
			}

			writeln!(message, "{nominal_count} nominal\n").unwrap();

			let mut last_status = None;
			for report in reports {
				let new_last = Some(report.status);
				if std::mem::replace(&mut last_status, new_last) != new_last {
					writeln!(message, "# {}:\n", report.status.as_repr()).unwrap();
				}
				let mut lines = report.message.lines();
				let first = lines.next().unwrap();
				writeln!(message, "- {first}").unwrap();
				for line in lines {
					writeln!(message, "  {line}").unwrap();
				}
				writeln!(message).unwrap();
			}

			match $args.output {
				Output::Mail => {
					eprintln!("reporting via mail ({critical_count} critical, {warning_count} warnings, {nominal_count} nominal)");
					if let Err(error) = send_mail("matt@felle.nz", &message) {
						eprintln!("error sending mail: {error}");
					}
				}
				Output::Stdout => {
					print!("{message}");
				}
			}
		} else {
			eprintln!("everything is nominal!");
		}
	};
}

enum Output {
	Mail,
	Stdout,
}

impl std::str::FromStr for Output {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"mail" => Self::Mail,
			"stdout" => Self::Stdout,
			_ => return Err("valid outputs are `mail` and `stdout`"),
		})
	}
}

/// Report system status.
#[derive(argh::FromArgs)]
struct Args {
	/// where to report the status to
	#[argh(option)]
	output: Output,
}

fn main() {
	let args: Args = argh::from_env();

	tests! {
		args;
		disk = crate::tests::disk::Test::new("/".as_ref()),
		failed_units = crate::tests::systemd::FailedUnitsTest::new(),
	}
}
