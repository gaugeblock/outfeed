//! Who is claiming a name, and why it cannot be defaulted (`GB-364`).
//!
//! crates.io fixes a name's metadata per published version and a version can never be deleted, so a placeholder
//! published with somebody else's homepage, license or repository is a permanent claim in their name. The three
//! fields were defaults on `ScaffoldOpts` once, and two of them had no flag - which is the shape of a field whose
//! author never expected a stranger to set it. They are resolved here, or the caller is told what to pass.

use std::path::Path;

use crate::error::OutfeedError;

/// The caller's identity, which a placeholder stamps permanently (`GB-364`).
///
/// crates.io fixes metadata per version and a version can never be deleted, so a placeholder published with
/// somebody else's homepage and license is a permanent, wrong-shaped claim. This type exists because the three
/// fields used to be defaults on [`ScaffoldOpts`]: a field with a default and no override is a field the author
/// never expected a stranger to set, and `homepage` and `license` were exactly that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Homepage written into every placeholder.
    pub homepage: String,
    /// SPDX license expression written verbatim, including `OR` expressions.
    pub license: String,
    /// Repository URL template; `{name}` is replaced with the crate name when present.
    pub repository_template: String,
}

impl Identity {
    /// Resolves identity from explicit flags, falling back to the caller's `origin` remote.
    ///
    /// Every field is required and none is invented: a flag wins, the git remote is the fallback for the two
    /// URL-shaped fields, and anything still missing is an error naming what to pass. Deriving the repository
    /// from a remote yields that URL unchanged for every crate, which is what a mono repository publishing
    /// several crates needs; a caller with one repository per crate passes a template containing `{name}`.
    ///
    /// # Errors
    ///
    /// [`OutfeedError::Reserve`] when a URL or the license cannot be established.
    pub fn resolve(
        homepage: Option<&str>,
        license: Option<&str>,
        repository: Option<&str>,
        start: &Path,
    ) -> Result<Self, OutfeedError> {
        let remote = git_remote_url(start);
        let repository_template = match repository.map(str::trim).filter(|v| !v.is_empty()) {
            Some(template) => template.to_string(),
            None => remote.clone().ok_or_else(|| {
                outfeed_error(
                    "--repository is required: no git remote `origin` was found to derive it from, and a \
                     placeholder's repository URL is permanent once published",
                )
            })?,
        };
        let homepage = match homepage.map(str::trim).filter(|v| !v.is_empty()) {
            Some(value) => value.to_string(),
            None => remote.ok_or_else(|| {
                outfeed_error(
                    "--homepage is required: no git remote `origin` was found to derive it from, and a \
                     placeholder's homepage is permanent once published",
                )
            })?,
        };
        let license = match license.map(str::trim).filter(|v| !v.is_empty()) {
            Some(value) => value.to_string(),
            None => {
                return Err(outfeed_error(
                    "--license is required: a placeholder's SPDX expression is permanent once published, and \
                     guessing it would license somebody else's crate",
                ));
            }
        };
        Ok(Self {
            homepage,
            license,
            repository_template,
        })
    }
}

/// The `origin` remote URL of the checkout containing `start`, if there is one.
fn git_remote_url(start: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(start)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if url.is_empty() {
        return None;
    }
    // `git@github.com:owner/repo.git` is a transport, not a URL a crate page can link to.
    Some(normalise_remote(&url))
}

/// Turns an scp-style or `.git`-suffixed remote into a browsable https URL.
fn normalise_remote(url: &str) -> String {
    let trimmed = url.strip_suffix(".git").unwrap_or(url);
    if let Some(rest) = trimmed.strip_prefix("git@") {
        let rest = rest.replacen(':', "/", 1);
        return format!("https://{rest}");
    }
    if let Some(rest) = trimmed.strip_prefix("ssh://git@") {
        return format!("https://{rest}");
    }
    trimmed.to_string()
}

/// One line an operator can act on.
fn outfeed_error(message: &str) -> OutfeedError {
    OutfeedError::Reserve(message.to_string())
}
