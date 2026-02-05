pub mod config;
pub mod constants;
pub mod logger;
pub mod normalizer;
pub mod utils;

// Re-export unicode normalization check functions
pub use unicode_normalization::{is_nfc, is_nfd};
