/// Errors that can occur in scene-world operations.
///
/// Using a typed enum (rather than `String`) makes this crate usable as a
/// pure Rust library without depending on `napi`. Call sites in `lib.rs`
/// convert `PhysicsError` into `napi::Error` at the FFI boundary.
#[derive(Debug)]
pub enum PhysicsError {
    /// No scene object exists for the given numeric entity ID.
    EntityNotFound(u32),
    /// A `position` or `target` vector did not have exactly 3 components.
    InvalidVec3,
    /// A `rotation` quaternion did not have exactly 4 components.
    InvalidRotation,
    /// A `direction` vector did not have exactly 3 components.
    InvalidDirection,
    /// An asset (GLTF/GLB) could not be loaded.
    AssetLoadError(String),
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsError::EntityNotFound(id) => {
                write!(f, "Scene object not found for entity ID: {id}")
            }
            PhysicsError::InvalidVec3 => {
                write!(f, "vector must have exactly 3 components [x, y, z]")
            }
            PhysicsError::InvalidRotation => {
                write!(f, "rotation must have exactly 4 components [x, y, z, w]")
            }
            PhysicsError::InvalidDirection => {
                write!(f, "direction must have exactly 3 components [x, y, z]")
            }
            PhysicsError::AssetLoadError(msg) => write!(f, "Asset load error: {msg}"),
        }
    }
}

impl std::error::Error for PhysicsError {}

impl From<PhysicsError> for napi::Error {
    fn from(e: PhysicsError) -> napi::Error {
        napi::Error::from_reason(e.to_string())
    }
}
