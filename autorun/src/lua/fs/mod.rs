// Filesystem for Autorun.
use once_cell::sync::Lazy;
use std::path::PathBuf;
use crate::configs;

struct FileSystem {
	base: PathBuf, // Base path ('autorun' directory)
}

impl Default for FileSystem {
	fn default() -> Self {
		Self {
			base: configs::path(configs::AUTORUN_PATH)
		}
	}
}

static FILESYSTEM: Lazy<FileSystem> = Lazy::new(|| FileSystem::default());