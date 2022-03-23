// Palette using iceberg-dark theme
// https://windowsterminalthemes.dev
type Rgb = (u8, u8, u8);

macro_rules! palette {
	( $name:ident : rgb($r:literal, $g:literal, $b:literal); $($rest:tt)* ) => {
		#[allow(unused)]
		pub const $name: Rgb = ($r, $g, $b);

		palette!( $($rest)* );
	};

	() => ();
}

palette! {
	BLACK: rgb(26, 33, 49);
	RED: rgb(226, 120, 120);
	GREEN: rgb(180, 190, 130);
	YELLOW: rgb(226, 164, 120);
	BLUE: rgb(132, 160, 198);
	PURPLE: rgb(160, 147, 199);
	CYAN: rgb(137, 184, 194);
	WHITE: rgb(198, 200, 209);
	BRIGHT_BLACK: rgb(107, 112, 137);
	BRIGHT_RED: rgb(233, 120, 120);
	BRIGHT_GREEN: rgb(192, 202, 142);
	BRIGHT_YELLOW: rgb(233, 176, 142);
	BRIGHT_BLUE: rgb(145, 172, 209);
	BRIGHT_PURPLE: rgb(172, 159, 210);
	BRIGHT_CYAN: rgb(149, 188, 206);
	BRIGHT_WHITE: rgb(210, 212, 221);
	BACKGROUND: rgb(22, 24, 33);
	FOREGROUND: rgb(198, 200, 209);
	SELECTION_BACKGROUND: rgb(198, 200, 209);
	CURSOR_COLOR: rgb(198, 200, 209);
}

/// println! macro using colors from the palette above and the ``colored`` crate.
/// Made this since the crate has no real elegant ways to do this itself.
/// printcol!(RED, "Error: {}", formatcol!(BLUE, "Foo"));
macro_rules! printcol {
	($name:ident, $msg:literal) => {
		println!(
			"{}",
			colored::Colorize::truecolor(
				$msg,
				$crate::ui::console::palette::$name.0,
				$crate::ui::console::palette::$name.1,
				$crate::ui::console::palette::$name.2
			)
		);
	};

	($name:ident, $fmt:literal, $($arg:tt)*) => {
		println!(
			"{}",
			colored::Colorize::truecolor(
				format!($fmt, $($arg)*).as_ref(),
				$crate::ui::console::palette::$name.0,
				$crate::ui::console::palette::$name.1,
				$crate::ui::console::palette::$name.2
			)
		);
	};

	($name:ident, $effect:ident, $fmt:literal, $($arg:tt)*) => {
		println!(
			"{}",
			colored::Colorize::truecolor(
				colored::Colorize::$effect(
					format!($fmt, $($arg)*).as_ref(),
				),
				$crate::ui::console::palette::$name.0,
				$crate::ui::console::palette::$name.1,
				$crate::ui::console::palette::$name.2
			)
		);
	};
}

/// format! macro using colors from the palette above and the ``colored`` crate.
/// Made this since the crate has no real elegant ways to do this itself.
/// formatcol!(RED, "Error: {}", formatcol!(BLUE, "Foo"));
macro_rules! formatcol {
	($name:ident, $msg:literal) => {
		format!(
			"{}",
			colored::Colorize::truecolor(
				$msg,
				$crate::ui::console::palette::$name.0,
				$crate::ui::console::palette::$name.1,
				$crate::ui::console::palette::$name.2
			)
		)
	};

	($name:ident, $effect:ident, $($arg:tt)+) => {
		format!(
			"{}",
			colored::Colorize::truecolor(
				colored::Colorize::$effect(
					std::fmt::format( format_args!( $($arg)+ ) ).as_ref(),
				),
				$crate::ui::console::palette::$name.0,
				$crate::ui::console::palette::$name.1,
				$crate::ui::console::palette::$name.2
			)
		)
	};

	($name:ident, $($arg:tt)+) => {
		format!(
			"{}",
			colored::Colorize::truecolor(
				std::fmt::format( format_args!( $($arg)+ ) ).as_ref(),
				$crate::ui::console::palette::$name.0,
				$crate::ui::console::palette::$name.1,
				$crate::ui::console::palette::$name.2
			)
		)
	};
}

/// ERROR foo bar
macro_rules! printerror {
	($effect:ident, $($arg:tt)+) => {
		println!(
			"{} {}",
			colored::Colorize::on_bright_red( colored::Colorize::white( colored::Colorize::bold(" ERROR ") ) ),
			$crate::ui::formatcol!(BRIGHT_WHITE, $effect, $($arg)+)
		)
	};
}

macro_rules! printwarning {
	($effect:ident, $($arg:tt)+) => {
		println!(
			"{} {}",
			colored::Colorize::on_bright_yellow( colored::Colorize::white( colored::Colorize::bold(" WARN ") ) ),
			$crate::ui::formatcol!(BRIGHT_WHITE, $effect, $($arg)+)
		)
	};
}

macro_rules! printinfo {
	($effect:ident, $($arg:tt)+) => {
		println!(
			"{} {}",
			colored::Colorize::on_bright_blue( colored::Colorize::white( colored::Colorize::bold(" INFO ") ) ),
			$crate::ui::formatcol!(BRIGHT_WHITE, $effect, $($arg)+)
		)
	};
}

#[allow(unused)]
macro_rules! printdebug {
	($effect:ident, $($arg:tt)+) => {
		println!(
			"{} {}",
			colored::Colorize::on_purple( colored::Colorize::white( colored::Colorize::bold(" DEBUG ") ) ),
			$crate::ui::formatcol!(BRIGHT_WHITE, $effect, $($arg)+)
		)
	};
}

pub(crate) use {printcol, formatcol, printinfo, printwarning, printdebug, printerror};