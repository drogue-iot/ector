//! Original reference https://github.com/embassy-rs/embassy/blob/e1ed492577d2151b4cc8ef995536dd045d2db087/embassy-hal-internal/src/drop.rs#L32

/// Panics if it is improperly disposed of.
///
/// This is to forbid cancelling a future/request.
///
/// To properly dispose, call the [defuse](Self::defuse) method before this object is dropped.
#[must_use = "to delay the drop bomb invokation to the end of the scope"]
pub struct DropBomb(());
impl DropBomb {
    pub fn new() -> Self {
        Self(())
    }

    /// Defuses the bomb, rendering it safe to drop.
    pub fn defuse(self) {
        core::mem::forget(self)
    }
}

impl Drop for DropBomb {
    fn drop(&mut self) {
        panic!("Dropped before the request completed. You  cannot cancel an ongoing request")
    }
}
