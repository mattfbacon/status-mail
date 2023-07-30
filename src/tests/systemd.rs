use std::fmt::Write;

use zbus::blocking::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::{Report, Status};

type Unit = (
	// name
	String,
	// description
	String,
	// load state
	String,
	// active state
	String,
	// sub state
	String,
	// "unit being followed" (???)
	String,
	// unit object path
	OwnedObjectPath,
	// queued job id
	u32,
	// job type
	String,
	// job object path
	OwnedObjectPath,
);

#[zbus::dbus_proxy(
	interface = "org.freedesktop.systemd1.Manager",
	default_service = "org.freedesktop.systemd1",
	default_path = "/org/freedesktop/systemd1",
	gen_async = false,
	gen_blocking = true
)]
trait Systemd {
	fn list_units_filtered(&self, states: &[&str]) -> zbus::Result<Vec<Unit>>;
}

pub struct FailedUnitsTest {}

impl FailedUnitsTest {
	pub fn new() -> Self {
		Self {}
	}
}

impl crate::Test for FailedUnitsTest {
	fn report(&self) -> anyhow::Result<Report> {
		let connection = Connection::system()?;
		let proxy = SystemdProxy::new(&connection)?;
		let units = proxy.list_units_filtered(&["failed"])?;

		if units.is_empty() {
			return Ok(Report {
				status: Status::Nominal,
				message: "All systemd units are happy".into(),
			});
		}

		let mut report = Report {
			status: Status::Critical,
			message: String::with_capacity(units.len() * 20),
		};

		writeln!(report.message, "{} systemd units have failed:", units.len()).unwrap();
		for (name, ..) in &units {
			writeln!(report.message, "- {name:?}").unwrap();
		}

		Ok(report)
	}
}
