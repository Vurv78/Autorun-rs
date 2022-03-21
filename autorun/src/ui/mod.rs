pub mod console;
pub(crate) use console::palette::{printcol, formatcol, printdebug, printinfo, printwarning, printerror};

pub fn init() {
	console::init();
}