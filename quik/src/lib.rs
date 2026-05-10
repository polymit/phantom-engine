//! `quik` is a Chrome 134 identity engine for `phantom-engine`.
//! See quik-architecture.md for implementation details of the 7-layer identity.

pub mod client;
pub mod error;
pub(crate) mod http2;
pub mod profile;
pub(crate) mod tls;

pub use crate::client::{connect, Client, Response};
pub use crate::error::{Error, Result};
pub use crate::profile::chrome_134::AKAMAI_FINGERPRINT;
pub use crate::profile::chrome_134::JA3_HASH;
pub use crate::profile::{ChromeProfile, Platform};
