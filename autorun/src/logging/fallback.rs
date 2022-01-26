/// Fallback bindings when logging is disabled

// Stderr
macro_rules! warn {
	($($arg:tt)*) => {
		eprintln!( $($arg)* )
	};
}

// Will never print (unless logging is enabled)
macro_rules! trace {
	($($arg:tt)*) => {
		()
	};
}

// Regular stdout
macro_rules! info {
	($($arg:tt)*) => {
		println!( $($arg)* )
	};
}

// Print to stderr
macro_rules! error {
	($($arg:tt)*) => {
		eprintln!( $($arg)* )
	};
}

// Only prints when in a debug build.
#[cfg(debug_assertions)]
macro_rules! debug {
	($($arg:tt)*) => {
		println!( $($arg)* )
	};
}

// We are in a release build, don't print anything.
#[cfg(not(debug_assertions))]
macro_rules! debug {
	($($arg:tt)*) => {
		()
	};
}

#[cfg(not(feature = "logging"))]
pub fn init() -> Result<(), LogInitError> {
	Ok(())
}
