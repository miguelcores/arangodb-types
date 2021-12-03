use lazy_static::lazy_static;
use tokio::sync::RwLock;

pub mod acquire;
pub mod acquire_aql;
pub mod acquire_list;
pub mod alive;
pub mod alive_list;
pub mod model;
pub mod release;
pub mod release_list;

lazy_static! {
    pub static ref TEST_RWLOCK: RwLock<()> = RwLock::new(());
}
