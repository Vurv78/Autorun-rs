use cfg_aliases::cfg_aliases;

fn main() {
	cfg_aliases! {
		win64: { all(target_os = "windows", target_arch = "x86_64") },
		linux64: { all(target_os = "linux", target_arch = "x86_64") },

		executor: { all(feature = "executor", win64) },
		inject: { all(feature = "inject", windows) },
		plugins: { all(feature = "plugins", windows) },
		colors: { all(feature = "colors", windows) },
		http: { all(feature = "http", windows) }
	}
}