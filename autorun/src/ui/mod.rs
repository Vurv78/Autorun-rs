pub mod console;
#[allow(unused)]
pub(crate) use console::palette::{
	formatcol, printcol, printdebug, printerror, printinfo, printwarning,
};

pub fn init() {
	console::init();
}
