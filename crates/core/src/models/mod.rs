/// Declares an enum that maps to a DB text column.
///
/// The string form is written once and drives serde, `as_str()` and `parse()`, so nothing
/// can drift between the wire format, the database value and the Rust variant. Agreement
/// with the `CHECK (col IN (...))` list in `migrations/001_schema.sql` is pinned by the
/// tests in `crates/core/src/tests.rs`.
macro_rules! db_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident => $value:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum $name {
            $(
                #[serde(rename = $value)]
                $variant,
            )+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The value stored in the database and sent on the wire.
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($name::$variant => $value),+
                }
            }

            /// Parse a database/wire value. `None` for anything unknown — callers decide
            /// the fallback rather than silently mapping it onto a wrong variant.
            pub fn parse(value: &str) -> Option<$name> {
                match value {
                    $($value => Some($name::$variant),)+
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

pub mod category;
pub mod match_thread;
pub mod post;
pub mod user;

pub use category::*;
pub use match_thread::*;
pub use post::*;
pub use user::*;
