pub use fs_mod_common::color::*;

// it's assumed elsewhere that an array of `u8` can be cast into an array of `Color`
static_assertions::assert_eq_align!([u8; 4], Color);
static_assertions::assert_eq_size!([u8; 4], Color);
