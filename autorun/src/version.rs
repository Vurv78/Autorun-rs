const REPO: &str = env!("CARGO_PKG_REPOSITORY");
const RELEASES_ENDPOINT: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/releases/latest");

/// Spawns another thread to check if Autorun is up to date.
pub fn check() {
	info!("Checking if Autorun is up to date...");
	std::thread::spawn(|| {
		let req = tinyget::get(RELEASES_ENDPOINT).with_timeout(5);

		if let Ok(x) = req.send() {
			match x.as_str() {
				Err(why) => error!("Failed to get latest release: {why}"),
				Ok(raw) => {
					// String in the form of major.minor.patch-beta(n) where -beta(n) is optional.
					// We want to break apart the major, minor and patch parts, and parse as integers.
					const VERSION_PHRASE: &str = "Autorun-rs v";
					if let Some(start) = raw.find(VERSION_PHRASE) {
						let pos = start + VERSION_PHRASE.len();
						let after = &raw[pos..pos + 12]; // Don't think versions will get very long.

						fn parse_semver(s: &str) -> Option<(u8, u8, u8, Option<u8>)> {
							let mut s = s;

							let end = s.find('.')?;
							let major = &s[..end].parse::<u8>().ok()?;
							s = &s[end + 1..];

							let end = s.find('.')?;
							let minor = &s[..end].parse::<u8>().ok()?;
							s = &s[end + 1..];

							let end = s.find(|x: char| !x.is_numeric()).unwrap_or(s.len());
							let patch = &s[..end].parse::<u8>().ok()?;

							let beta = if end <= s.len() {
								s = &s[end + 1..];
								if s.contains("-beta") {
									let end = s.find(|x: char| !x.is_numeric())?;
									if let Ok(n) = &s[end + 1..].parse::<u8>() {
										Some(*n)
									} else {
										Some(0)
									}
								} else {
									None
								}
							} else {
								None
							};

							Some((*major, *minor, *patch, beta))
						}

						if let Some(latest_version) = parse_semver(after) {
							let semver = format!("{}   ", env!("CARGO_PKG_VERSION"));
							if let Some(local_version) = parse_semver(&semver) {
								// Compare latest_version and local_version and see if local_version is outdated
								let outdated = {
									// 2.0.3 vs 3.1.2
									local_version.0 < latest_version.0 ||

									// 1.3.2 vs 1.4.0
									local_version.1 < latest_version.1 ||

									// 1.2.3 vs 1.2.4
									local_version.2 < latest_version.2 ||

									// 1.2.3-beta vs 1.2.3
									local_version.3.is_some() && latest_version.3.is_none()
								};
								if outdated {
									info!(
										"New update found: v{}.{}.{}! You are on v{}.{}.{}\nUpdate here! {}",
										latest_version.0, latest_version.1, latest_version.2,
										local_version.0, local_version.1, local_version.2,

										REPO
									)
								} else {
									info!("Autorun is up to date.");
								}
							}
						} else {
							error!("Failed to parse semver in release. Report this on github :(");
						}
					} else {
						error!("Failed to parse latest release data");
					}
				}
			}
		} else {
			error!("Failed to check for latest version")
		}
	});
}
