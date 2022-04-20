use cfg_aliases::cfg_aliases;

fn main() {
	cfg_aliases! {
		executor: { all(feature = "executor", target_os = "windows", target_arch = "x86_64") },
		inject: { all(feature = "inject", target_os = "windows") },
		plugins: { all(feature = "plugins", target_os = "windows") },
		colors: { all(feature = "colors", target_os = "windows") },
		http: { all(feature = "http", target_os = "windows") }
	}
}