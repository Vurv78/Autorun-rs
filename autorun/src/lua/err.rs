use rglua::cstr;

// Error Messages
pub const INVALID_LOG_LEVEL: *const i8 =
	cstr!("Invalid log level (Should be 1-5, 1 being Error, 5 being Trace)");
