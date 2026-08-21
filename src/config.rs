use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    string::FromUtf8Error,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not read config file `{}': {}", filename.to_string_lossy(), source)]
    ReadConfigFile {
        filename: PathBuf,
        source: io::Error,
    },

    #[error("Could not parse config file `{}': {}", filename.to_string_lossy(), source)]
    ParseConfigFile {
        filename: PathBuf,
        source: toml::de::Error,
    },

    #[error("Can only specify one of `fqdn' or `session_url' in the same config")]
    FqdnOrSessionUrl {},

    #[error("Must specify at least 1 for `concurrent_downloads'")]
    ConcurrentDownloadsIsZero {},

    #[error("`directory_separator' must not be empty")]
    EmptyDirectorySeparator {},

    #[error("Custom keyword `{keyword}' for tag `{tag}' is invalid; keywords must start with `$' and use lowercase [a-z0-9_-] only")]
    InvalidCustomKeyword { tag: String, keyword: String },

    #[error("Duplicate custom keyword mapping for `{keyword}'")]
    DuplicateCustomKeyword { keyword: String },

    #[error("Custom keyword mapping for tag `{tag}' collides with built-in tag mappings")]
    CustomKeywordTagCollision { tag: String },

    #[error(
        "Custom keyword mapping for keyword `{keyword}' collides with built-in keyword mappings"
    )]
    CustomKeywordValueCollision { keyword: String },

    #[error("Could not execute password command: {}", source)]
    ExecutePasswordCommand { source: io::Error },

    #[error("Password command exited with `{}': {}", status, stderr)]
    PasswordCommandStatus { status: ExitStatus, stderr: String },

    #[error("Could not decode password command output as utf-8")]
    DecodePasswordCommand { source: FromUtf8Error },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Username for basic HTTP authentication.
    pub username: String,

    /// Shell command which will print a password to stdout for basic HTTP authentication.
    pub password_command: String,

    /// Fully qualified domain name of the JMAP service.
    ///
    /// mujmap looks up the JMAP SRV record for this host to determine the JMAP session URL.
    /// Mutually exclusive with `session_url`.
    pub fqdn: Option<String>,

    /// Session URL to connect to.
    ///
    /// Mutually exclusive with `fqdn`.
    pub session_url: Option<String>,

    /// Number of email files to download in parallel.
    ///
    /// This corresponds to the number of blocking OS threads that will be created for HTTP download
    /// requests. Increasing this number too high will likely result in many failed connections.
    #[serde(default = "default_concurrent_downloads")]
    pub concurrent_downloads: usize,

    /// Number of seconds before timing out on a stalled connection.
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Number of retries to download an email file. 0 means infinite.
    #[serde(default = "default_retries")]
    pub retries: usize,

    /// Whether to create new mailboxes automatically on the server from notmuch tags.
    #[serde(default = "default_auto_create_new_mailboxes")]
    pub auto_create_new_mailboxes: bool,

    /// If true, convert all DOS newlines in downloaded mail files to Unix newlines.
    #[serde(default = "default_convert_dos_to_unix")]
    pub convert_dos_to_unix: bool,

    /// The cache directory in which to store mail files while they are being downloaded. The
    /// default is operating-system specific.
    #[serde(default = "Default::default")]
    pub cache_dir: Option<PathBuf>,

    /// Customize the names and synchronization behaviors of notmuch tags with JMAP keywords and
    /// mailboxes.
    #[serde(default = "Default::default")]
    pub tags: Tags,
}

#[derive(Debug, Deserialize)]
pub struct Tags {
    /// Translate all mailboxes to lowercase names when mapping to notmuch tags.
    ///
    /// Defaults to `false`.
    #[serde(default = "default_lowercase")]
    pub lowercase: bool,

    /// Directory separator for mapping notmuch tags to maildirs.
    ///
    /// Defaults to `"/"`.
    #[serde(default = "default_directory_separator")]
    pub directory_separator: String,

    /// Tag for notmuch to use for messages stored in the mailbox labeled with the [Inbox name
    /// attribute](https://www.rfc-editor.org/rfc/rfc8621.html).
    ///
    /// If set to an empty string, this mailbox *and its child mailboxes* are not synchronized with
    /// a tag.
    ///
    /// Defaults to `"inbox"`.
    #[serde(default = "default_inbox")]
    pub inbox: String,

    /// Tag for notmuch to use for messages stored in the mailbox labeled with the [Trash name
    /// attribute](https://www.rfc-editor.org/rfc/rfc6154.html).
    ///
    /// This configuration option is called `deleted` instead of `trash` because notmuch's UIs all
    /// prefer "deleted" by default.
    ///
    /// If set to an empty string, this mailbox *and its child mailboxes* are not synchronized with
    /// a tag.
    ///
    /// Defaults to `"deleted"`.
    #[serde(default = "default_deleted")]
    pub deleted: String,

    /// Tag for notmuch to use for messages stored in the mailbox labeled with the [`Sent` name
    /// attribute](https://www.rfc-editor.org/rfc/rfc6154.html).
    ///
    /// If set to an empty string, this mailbox *and its child mailboxes* are not synchronized with
    /// a tag.
    ///
    /// Defaults to `"sent"`.
    #[serde(default = "default_sent")]
    pub sent: String,

    /// Tag for notmuch to use for messages stored in the mailbox labeled with the [`Junk` name
    /// attribute](https://www.rfc-editor.org/rfc/rfc8621.html) and/or with the [`$Junk`
    /// keyword](https://www.iana.org/assignments/imap-jmap-keywords/junk/junk-template), except for
    /// messages with the [`$NotJunk`
    /// keyword](https://www.iana.org/assignments/imap-jmap-keywords/notjunk/notjunk-template).
    ///
    /// The combination of these three traits becomes a bit tangled, so further explanation is
    /// warranted. Most email services in the modern day, especially those that support JMAP,
    /// provide a dedicated "Spam" or "Junk" mailbox which has the `Junk` name attribute mentioned
    /// above. However, there may exist services which do not have this mailbox, but still support
    /// the `$Junk` and `$NotJunk` keywords. mujmap behaves in the following way:
    ///
    /// * If the mailbox exists, it becomes the sole source of truth. mujmap will entirely disregard
    ///   the `$Junk` and `$NotJunk` keywords. * If the mailbox does not exist, messages with the
    ///   `$Junk` keyword *that do not also have* a `$NotJunk` keyword are tagged as spam. When
    ///   pushing, both `$Junk` and `$NotJunk` are set appropriately.
    ///
    /// This configuration option is called `spam` instead of `junk` despite all of the
    /// aforementioned specifications preferring "junk" because notmuch's UIs all prefer "spam" by
    /// default.
    ///
    /// If set to an empty string, this mailbox, *its child mailboxes*, and these keywords are not
    /// synchronized with a tag.
    ///
    /// Defaults to `"spam"`.
    #[serde(default = "default_spam")]
    pub spam: String,

    /// Tag for notmuch to use for messages stored in the mailbox labeled with the [`Important` name
    /// attribute](https://www.rfc-editor.org/rfc/rfc8457.html) and/or with the [`$Important`
    /// keyword](https://www.rfc-editor.org/rfc/rfc8457.html).
    ///
    /// * If a mailbox with the `Important` role exists, this is used as the sole source of truth
    ///   when pulling for tagging messages as "important". * If not, the `$Important` keyword is
    ///   considered instead. * In both cases, the `$Important` keyword is set on the server when
    ///   pushing. In the first case, it's also copied to the `Important` mailbox.
    ///
    /// If set to an empty string, this mailbox, *its child mailboxes*, and this keyword are not
    /// synchronized with a tag.
    ///
    /// Defaults to `"important"`.
    #[serde(default = "default_important")]
    pub important: String,

    /// Tag for notmuch to use for the [IANA `$Phishing`
    /// keyword](https://www.iana.org/assignments/imap-jmap-keywords/phishing/phishing-template).
    ///
    /// If set to an empty string, this keyword is not synchronized with a tag.
    ///
    /// Defaults to `"phishing"`.
    #[serde(default = "default_phishing")]
    pub phishing: String,

    /// Mapping of notmuch tag to JMAP keyword.
    ///
    /// Keywords must start with `$` and use lowercase [a-z0-9_-] only.
    #[serde(default = "Default::default")]
    pub custom_keywords: HashMap<String, String>,
}

impl Default for Tags {
    fn default() -> Self {
        Self {
            lowercase: default_lowercase(),
            directory_separator: default_directory_separator(),
            inbox: default_inbox(),
            deleted: default_deleted(),
            sent: default_sent(),
            spam: default_spam(),
            important: default_important(),
            phishing: default_phishing(),
            custom_keywords: Default::default(),
        }
    }
}

impl Tags {
    fn validate(&self) -> Result<()> {
        let mut seen_keywords: HashSet<&str> = HashSet::new();
        for (tag, keyword) in &self.custom_keywords {
            if !is_valid_custom_keyword(keyword) {
                Err(Error::InvalidCustomKeyword {
                    tag: tag.clone(),
                    keyword: keyword.clone(),
                })?;
            }
            if !seen_keywords.insert(keyword.as_str()) {
                Err(Error::DuplicateCustomKeyword {
                    keyword: keyword.clone(),
                })?;
            }
            if self.builtin_keyword_tags().contains(tag.as_str()) {
                Err(Error::CustomKeywordTagCollision { tag: tag.clone() })?;
            }
            if self.builtin_keywords().contains(keyword.as_str()) {
                Err(Error::CustomKeywordValueCollision {
                    keyword: keyword.clone(),
                })?;
            }
        }
        Ok(())
    }

    pub fn builtin_keyword_tags(&self) -> HashSet<&str> {
        [
            "draft",
            "unread",
            "flagged",
            "replied",
            "passed",
            self.spam.as_str(),
            self.important.as_str(),
            self.phishing.as_str(),
        ]
        .into_iter()
        .filter(|x| !x.is_empty())
        .collect()
    }

    fn builtin_keywords(&self) -> HashSet<&str> {
        [
            "$draft",
            "$seen",
            "$flagged",
            "$answered",
            "$forwarded",
            "$junk",
            "$notjunk",
            "$important",
            "$phishing",
        ]
        .into_iter()
        .collect()
    }
}

fn is_valid_custom_keyword(keyword: &str) -> bool {
    let Some(rest) = keyword.strip_prefix('$') else {
        return false;
    };
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn default_lowercase() -> bool {
    false
}

fn default_directory_separator() -> String {
    "/".to_owned()
}

fn default_inbox() -> String {
    "inbox".to_owned()
}

fn default_deleted() -> String {
    "deleted".to_owned()
}

fn default_sent() -> String {
    "sent".to_owned()
}

fn default_spam() -> String {
    "spam".to_owned()
}

fn default_important() -> String {
    "important".to_owned()
}

fn default_phishing() -> String {
    "phishing".to_owned()
}

fn default_concurrent_downloads() -> usize {
    8
}

fn default_timeout() -> u64 {
    5
}

fn default_retries() -> usize {
    5
}

fn default_auto_create_new_mailboxes() -> bool {
    true
}

fn default_convert_dos_to_unix() -> bool {
    true
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents =
            fs::read_to_string(path.as_ref()).map_err(|source| Error::ReadConfigFile {
                filename: path.as_ref().into(),
                source,
            })?;
        let config: Self =
            toml::from_str(contents.as_str()).map_err(|source| Error::ParseConfigFile {
                filename: path.as_ref().into(),
                source,
            })?;

        // Perform final validation.
        if (config.fqdn.is_some() && config.session_url.is_some()) {
            Err(Error::FqdnOrSessionUrl {})?
        }
        if config.concurrent_downloads < 1 {
            Err(Error::ConcurrentDownloadsIsZero {})?
        };
        if config.tags.directory_separator.is_empty() {
            Err(Error::EmptyDirectorySeparator {})?
        };
        config.tags.validate()?;
        Ok(config)
    }

    pub fn password(&self) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(self.password_command.as_str())
            .output()
            .map_err(|source| Error::ExecutePasswordCommand { source })?;
        if !output.status.success() {
            Err(Error::PasswordCommandStatus {
                status: output.status,
                stderr: String::from_utf8(output.stderr)
                    .unwrap_or_else(|e| format!("<utf-8 decode error: {e}>")),
            })?
        };
        let stdout = String::from_utf8(output.stdout)
            .map_err(|source| Error::DecodePasswordCommand { source })?;
        Ok(stdout.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{is_valid_custom_keyword, Config, Error};
    use std::collections::HashMap;

    fn base_config() -> Config {
        Config {
            username: "user@example.com".to_string(),
            password_command: "echo pw".to_string(),
            fqdn: None,
            session_url: None,
            concurrent_downloads: 8,
            timeout: 5,
            retries: 5,
            auto_create_new_mailboxes: true,
            convert_dos_to_unix: true,
            cache_dir: None,
            tags: Default::default(),
        }
    }

    #[test]
    fn custom_keyword_format_is_validated() {
        assert!(is_valid_custom_keyword("$foo"));
        assert!(is_valid_custom_keyword("$foo-1_bar"));
        assert!(!is_valid_custom_keyword("foo"));
        assert!(!is_valid_custom_keyword("$Foo"));
    }

    #[test]
    fn custom_keyword_parses_from_config() {
        let config: Config = toml::from_str(
            r#"
username = "user@example.com"
password_command = "echo pw"

[tags.custom_keywords]
todo = "$todo"
"#,
        )
        .unwrap();

        assert_eq!(
            config.tags.custom_keywords.get("todo"),
            Some(&"$todo".to_string())
        );
    }

    #[test]
    fn custom_keyword_rejects_builtins() {
        let mut config = base_config();
        config
            .tags
            .custom_keywords
            .insert("todo".to_string(), "$seen".to_string());
        let err = config.tags.validate().unwrap_err();
        assert!(matches!(err, Error::CustomKeywordValueCollision { .. }));
    }

    #[test]
    fn custom_keyword_rejects_builtin_tags() {
        let mut config = base_config();
        config
            .tags
            .custom_keywords
            .insert("draft".to_string(), "$todo".to_string());
        let err = config.tags.validate().unwrap_err();
        assert!(matches!(err, Error::CustomKeywordTagCollision { .. }));
    }

    #[test]
    fn custom_keyword_rejects_duplicate_keyword_targets() {
        let mut config = base_config();
        config.tags.custom_keywords = HashMap::from([
            ("todo".to_string(), "$project".to_string()),
            ("work".to_string(), "$project".to_string()),
        ]);
        let err = config.tags.validate().unwrap_err();
        assert!(matches!(err, Error::DuplicateCustomKeyword { .. }));
    }
}
