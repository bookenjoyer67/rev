// A3.2: `alliances` stays declared — `federation/mod.rs` still uses it, and federation is B's
// to delete. `communities` is gone with its table.
pub mod alliances;
pub mod conversations;
pub mod endorsements;
pub mod notifications;
pub mod posts;
pub mod reports;
pub mod sessions;
pub mod users;
