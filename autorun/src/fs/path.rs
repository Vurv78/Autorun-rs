// Class abstracting over a PathBuf

use super::in_autorun;
use std::{
	ops::Deref,
	path::{Path, PathBuf},
};

#[derive(Debug)]
#[repr(transparent)]
pub struct FSPath(PathBuf);

// Implement everything from the original PathBuf type, except they call in_autorun for every call to the filesystem.
impl FSPath {
	pub fn from<P: AsRef<Path>>(p: P) -> Self {
		Self(p.as_ref().to_path_buf())
	}

	pub fn is_dir(&self) -> bool {
		in_autorun(&self.0).is_dir()
	}

	pub fn exists(&self) -> bool {
		in_autorun(&self.0).exists()
	}

	pub fn join(&self, path: impl AsRef<Path>) -> Self {
		Self(self.0.join(path))
	}

	pub fn to_owned(&self) -> Self {
		Self(self.0.clone())
	}

	pub fn parent(&self) -> Option<Self> {
		self.0.parent().map(|x| Self(x.to_path_buf()))
	}

	pub fn pop(&mut self) -> bool {
		self.0.pop()
	}
}

impl Deref for FSPath {
	type Target = PathBuf;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsRef<Path> for FSPath {
	fn as_ref(&self) -> &Path {
		self.0.as_ref()
	}
}
