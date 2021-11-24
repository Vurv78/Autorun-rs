#[macro_export]
macro_rules! lazy_detour {
	// Lazy Path
	( $vis:vis static $name:ident : $t:ty = ($target:expr, $tour:expr) ; $($rest:tt)* ) => {
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
	// OnceCell Path
	( $vis:vis static $name:ident : $t:ty ; $($rest:tt)* ) => {
		$vis static $name: once_cell::sync::OnceCell<detour::GenericDetour<$t>> = once_cell::sync::OnceCell::new();
		lazy_detour!( $($rest)* );
	};
	() => ();
}