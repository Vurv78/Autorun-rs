macro_rules! lazy_detour {
	// Lazy Path
	( $(#[$m:meta])* $vis:vis static $name:ident : $t:ty = ($target:expr, $tour:expr) ; $($rest:tt)* ) => {
		$(#[$m])*
		$vis static $name: once_cell::sync::Lazy< detour::GenericDetour< $t > > = once_cell::sync::Lazy::new(|| unsafe {
			match detour::GenericDetour::new( $target, $tour ) {
				Ok(b) => {
					b.enable().expect( concat!("Failed to enable detour '", stringify!($name), "'") );
					b
				},
				Err(why) => panic!( concat!("Failed to create hook '", stringify!($name), "' {}"), why)
			}
		});
		lazy_detour!( $($rest)* );
	};
	() => ();
}
pub(crate) use lazy_detour;