/// Autorun-gui
/// This is a gui to run alongside the ``gui`` version of autorun.
/// It works by having a server (autorun) client (autorun-gui) connection as to not disrupt the main thread (gmod)
/// This is a workaround for Dioxus and every other rust library that doesn't support running outside main thread, which would freeze gmod.

/*
mod logging;
mod app;
mod io;
*/

// Start app in current thread
pub fn main() {
	// logging::init();

	// app::launch();
	// App will launch io
}