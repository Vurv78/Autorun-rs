#[cfg(not(feature = "gui"))]
mod console;

#[cfg(feature = "gui")]
mod gui;

pub fn init() {
	#[cfg(not(feature = "gui"))]
	console::init();

	#[cfg(feature = "gui")]
	gui::init();
}