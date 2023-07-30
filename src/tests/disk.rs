use std::path::Path;

use anyhow::Context as _;

use crate::{Report, Status};

pub struct Test<'a> {
	disk: &'a Path,
}

struct Stats {
	size: u64,
	use_percentage: f32,
}

impl<'a> Test<'a> {
	pub fn new(disk: &'a Path) -> Self {
		Self { disk }
	}

	#[allow(clippy::cast_precision_loss, /* not a precise calculation */)]
	fn get_stats(&self) -> anyhow::Result<Stats> {
		let raw_stats =
			nix::sys::statvfs::statvfs(self.disk).with_context(|| format!("statvfs({:?})", self.disk))?;
		let blocks = raw_stats.blocks();
		let size = raw_stats.blocks() * raw_stats.fragment_size();
		let use_percentage = ((blocks - raw_stats.blocks_available()) * 1000 / blocks) as f32 / 10.0;
		Ok(Stats {
			size,
			use_percentage,
		})
	}
}

impl crate::Test for Test<'_> {
	fn report(&self) -> anyhow::Result<Report> {
		let Stats {
			size,
			use_percentage,
		} = self.get_stats()?;

		let status = if use_percentage > 90.0 {
			Status::Critical
		} else if use_percentage > 75.0 {
			Status::Warning
		} else {
			Status::Nominal
		};

		let disk = self.disk;
		let size = humansize::SizeFormatter::new(size, humansize::BINARY.decimal_places(0));
		Ok(Report {
			status,
			message: format!("{use_percentage:.1}% of {size} is in use on {disk:?}"),
		})
	}
}
