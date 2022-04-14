use fs_err as fs;
use std::{
	ffi::CStr,
	path::PathBuf,
	sync::{Arc, Mutex},
	time::Duration,
};

use crate::{configs::SETTINGS, fs as afs};
use once_cell::sync::Lazy;

use super::DispatchParams;

struct DumpEntry {
	path: PathBuf,
	content: String,
}

static DUMP_QUEUE: Lazy<Arc<Mutex<Vec<DumpEntry>>>> =
	Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

fn strip_invalid(str: &str) -> String {
	let mut pat = lua_patterns::LuaPattern::new(r#"[:<>"|?*]"#);
	pat.gsub(str, "_")
}

/// Will only be run if filesteal is enabled.
pub fn dump(params: &mut DispatchParams) {
	if params.path.len() < 1000 {
		// Ignore paths that are ridiculously long
		if let Ok(mut queue) = DUMP_QUEUE.try_lock() {
			let mut fmt = SETTINGS.filesteal.format.clone();

			if fmt.contains("<ip>") {
				let ip = unsafe { CStr::from_ptr(params.ip) };
				let ip = ip.to_string_lossy();

				fmt = fmt.replace("<ip>", &ip);
			}

			if fmt.contains("<hostname>") {
				let hostname = params.net.GetName();
				let hostname = unsafe { CStr::from_ptr(hostname) };
				let hostname = hostname.to_string_lossy();

				fmt = fmt.replace("<hostname>", &hostname);
			}

			let (code, _) = params.get_code();
			let code = unsafe { CStr::from_ptr(code) };
			let code = code.to_string_lossy().to_string();

			fmt = strip_invalid(&fmt);

			let path_clean = strip_invalid(params.path);
			let path = PathBuf::from(&fmt).join(path_clean).with_extension("lua");

			queue.push(DumpEntry {
				path,
				content: code,
			});
		}
	}
}

const QUEUE_COOLDOWN: Duration = Duration::from_millis(300);

pub fn queue() {
	// Same deal as the lua executor. Run in a separate thread and endlessly loop pulling latest files to dump
	loop {
		std::thread::sleep(QUEUE_COOLDOWN);

		if let Ok(mut queue) = DUMP_QUEUE.try_lock() {
			if !queue.is_empty() {
				// Handle 15 files at a time max
				// 15 files every 300 ms is around 50 files per sec, not bad
				let len = 15.min(queue.len());
				let dump_dir = &*afs::in_autorun(afs::DUMP_DIR);
				for entry in queue.drain(..len) {
					let path = dump_dir.join(entry.path);
					let content = entry.content;

					let p = path.parent().unwrap_or(&path);
					if !p.exists() {
						if let Err(why) = fs::create_dir_all(&p) {
							debug!("Failed to create directory {}: {}", p.display(), why);
						}
					}

					if let Err(why) = fs::write(&path, content) {
						error!("Failed to write to {}: {}", path.display(), why);
					}
				}
			}
		} else {
			debug!("Failed to lock dump queue");
		}
	}
}

/// Create async queue to dump files
pub fn start_queue() {
	std::thread::spawn(queue);
}
