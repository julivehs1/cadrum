//! Which kernel produced a piece of geometry.

/// **A stamp for consumers that cache geometry on disk across runs.**
///
/// A cached body or mesh is only valid for the kernel that made it. Mix this
/// into the cache key and a changed kernel invalidates the cache by itself,
/// instead of relying on someone remembering to clear it.
///
/// It covers `cpp/wrapper.cpp`, `cpp/wrapper.h`, `src/occt/ffi.rs`, the
/// resolved OCCT root and the crate version — see `emit_kernel_stamp` in
/// `build.rs` for why the version alone would not do.
///
/// The value is stable for a given build and carries no meaning beyond
/// equality: two runs of the same binary always agree, two different kernels
/// practically never do.
pub fn kernel_stamp() -> &'static str {
	env!("CADRUM_KERNEL_STAMP")
}
