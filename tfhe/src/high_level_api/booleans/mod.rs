pub use base::{FheBool, FheBoolConformanceParams};
pub use compressed::CompressedFheBool;

mod base;
mod compressed;
mod encrypt;
pub(crate) mod inner;
#[cfg(test)]
mod tests;
