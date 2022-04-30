use fs_err as fs;
use std::{
	ffi::CStr,
	path::PathBuf,
	sync::{Arc, Mutex}, time::Duration
};

use crate::{configs::SETTINGS, fs as afs};
use once_cell::sync::Lazy;

use super::DispatchParams;

pub struct DumpEntry {
	path: PathBuf,
	content: String,
}

pub static DUMP_QUEUE: Lazy<Arc<Mutex<Vec<DumpEntry>>>> =
Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

fn strip_invalid(str: &str) -> String {
	let mut pat = lupat::Pattern::<'_, 1>::new(r#"[:<>"|?*]"#).unwrap();
	pat.gsub(str, "_").unwrap()
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

const BASE_COOLDOWN: u64 = 300;
const CHUNK_COOLDOWN: Duration = Duration::from_secs(20); // After queue was delegated into chunks.
const FAIL_COOLDOWN: Duration = Duration::from_secs(5); // Failed to lock mutex
const FILES_PER_TICK: usize = 10;

const CHUNKS: u8 = 5;

// Require there to at least be 512 * CHUNKS files (2500) to dump to start using chunks.
const CHUNK_SIZE: usize = 512;
const MIN_TOTAL_CHUNKS: usize = 512 * CHUNKS as usize;

enum Act {
	Fail,
	Chunk,
	Normal
}

pub fn queue() {
	// Same deal as the lua executor. Run in a separate thread and endlessly loop pulling latest files to dump
	loop {
		let mut size = 0u64;
		let act;

		if let Ok(mut queue) = DUMP_QUEUE.try_lock() {
			if queue.is_empty() { continue };

			let queue_len = queue.len();
			let dump_dir = &*afs::in_autorun(afs::DUMP_DIR);

			fn handle_entry(dump_dir: &std::path::Path, entry: &DumpEntry) -> u64 {
				let path = dump_dir.join(&entry.path);

				let p = path.parent().unwrap_or(&path);
				if !p.exists() {
					if let Err(why) = fs::create_dir_all(&p) {
						debug!("Failed to create directory {}: {}", p.display(), why);
					}
				}

				if let Err(why) = fs::write(&path, &entry.content) {
					error!("Failed to write to {}: {}", path.display(), why);
				}

				entry.content.len() as u64
			}

			if queue_len < MIN_TOTAL_CHUNKS {
				// Less than minimum, stay on one thread.
				let len = FILES_PER_TICK.min(queue_len);

				for entry in queue.drain(..len) {
					size += handle_entry(dump_dir, &entry);
				}

				act = Act::Normal;
			} else {
				use rayon::prelude::*;

				// Carve up work into pieces to speed up work.
				size += queue.par_chunks(CHUNK_SIZE).map(|chunk| {
					let mut size = 0;
					for entry in chunk {
						size += handle_entry(dump_dir, entry);
					}
					size
				}).sum::<u64>();

				queue.drain(..MIN_TOTAL_CHUNKS);

				act = Act::Chunk;
			}
		} else {
			debug!("Failed to lock dump queue");
			std::thread::sleep(FAIL_COOLDOWN);

			act = Act::Fail;
		}

		let time = match act {
			Act::Fail => FAIL_COOLDOWN,
			Act::Chunk => CHUNK_COOLDOWN,

			// Every 100 bytes/length, add an extra millisecond to wait for next loop.
			// So 1kb would be 20ms, 100kb would be 2 seconds, 1mb 20 seconds
			Act::Normal => Duration::from_millis(size / 500 + BASE_COOLDOWN)
		};

		std::thread::sleep(time);
	}
}

/// Create async queue to dump files
pub fn start_queue() {
	std::thread::spawn(queue);
}
