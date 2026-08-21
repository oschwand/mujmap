mod request;
mod response;
mod session;

use core::fmt;

pub use request::*;
pub use response::*;
use serde::{Deserialize, Serialize};
pub use session::*;

#[derive(Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Clone)]
pub struct Id(pub String);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Clone)]
pub struct State(pub String);

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Keywords that assign meaning to email.
///
/// Note that JMAP mandates that these be lowercase.
///
/// See <https://www.iana.org/assignments/imap-jmap-keywords/imap-jmap-keywords.xhtml>.
pub type EmailKeyword = String;

pub const EMAIL_KEYWORD_DRAFT: &str = "$draft";
pub const EMAIL_KEYWORD_SEEN: &str = "$seen";
pub const EMAIL_KEYWORD_FLAGGED: &str = "$flagged";
pub const EMAIL_KEYWORD_ANSWERED: &str = "$answered";
pub const EMAIL_KEYWORD_FORWARDED: &str = "$forwarded";
pub const EMAIL_KEYWORD_JUNK: &str = "$junk";
pub const EMAIL_KEYWORD_NOT_JUNK: &str = "$notjunk";
pub const EMAIL_KEYWORD_PHISHING: &str = "$phishing";
pub const EMAIL_KEYWORD_IMPORTANT: &str = "$important";
