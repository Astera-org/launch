use std::sync::OnceLock;

pub trait OffsetDateTimeExt {
    /// Convenience method that calls [`time::OffsetDateTime::to_offset`] with the return value of
    /// [`time::UtcOffset::current_local_offset`]. The current local offset is cached upon the first call. This call may
    /// have to be made before the program spawns threads. Browse the source code of
    /// [`time::UtcOffset::current_local_offset`] to understand why.
    fn to_local(self) -> time::Result<time::OffsetDateTime>;
}

pub fn local_offset() -> Result<time::UtcOffset, time::error::IndeterminateOffset> {
    static CACHE: OnceLock<Result<time::UtcOffset, time::error::IndeterminateOffset>> =
        OnceLock::new();
    *CACHE.get_or_init(time::UtcOffset::current_local_offset)
}

impl OffsetDateTimeExt for time::OffsetDateTime {
    fn to_local(self) -> time::Result<time::OffsetDateTime> {
        Ok(self.to_offset(local_offset()?))
    }
}
