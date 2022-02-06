// Mutexes --------------------------------------------------------------------
// 2 minutes in seconds
#[cfg(not(feature = "test"))]
pub const MUTEX_ALIVE_INTERVAL: u64 = 2 * 60;
#[cfg(feature = "test")]
pub const MUTEX_ALIVE_INTERVAL: u64 = 3;
// Alive interval + 10 seconds
pub const MUTEX_EXPIRATION: u64 = MUTEX_ALIVE_INTERVAL + 10;
// From 50ms to 150ms
pub const MUTEX_ACQUIRE_MIN_INTERVAL: u64 = 100;
pub const MUTEX_ACQUIRE_MAX_INTERVAL: u64 = 150;
