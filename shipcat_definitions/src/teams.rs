use crate::structs::SlackChannel;
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;
use super::Result;

/// Information on one human
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    /// Name in "firstname.lastname" format (must match filename)
    pub name: String,
    /// A github user id (alphanumeric)
    pub github: String,
    /// A slack guid (U...)
    pub slack: String,
    /// An email "firstname.lastname@babylonhealth.com"
    pub email: String,
}

/// Information about a Squad of humans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Squad {
    /// Dash-separated, lower-case name of the squad
    pub name: String,
    /// List of lowercase, dot-separated members of the squad
    pub members: Vec<String>,
    /// Github team references for the squad
    pub github: GithubTeams,
    /// Slack channels for the squad
    pub slack: SlackSet,
}

/// Information about a Tribe of squads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tribe {
    /// Dash-separated, lower-case name of the tribe
    pub name: String,
    /// List of lowercase, dash-separated squads in the tribe
    pub squads: Vec<String>,
    /// Github team references for the tribe
    pub github: Option<GithubTeams>,
    /// Slack channels for the tribe
    pub slack: Option<SlackSet>,
}

/// Team data combined into a single structure
///
/// Contains all data from all 4 folders in a EWOK_TEAMS_DIR
/// All entries are sorted by filename (.name properties)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Owners {
    /// All people in people/{key}.toml
    pub people: BTreeMap<String, Person>,
    /// All squads in squads/{key}.toml
    pub squads: BTreeMap<String, Squad>,
    /// All tribes in tribes/{key}.toml
    pub tribes: BTreeMap<String, Tribe>,
}

impl Owners {
    /// Read a config in pwd and leave placeholders
    pub fn read() -> Result<Owners> {
        let pwd = Path::new(".");
        Owners::read_from(&pwd.to_path_buf())
    }

    pub fn read_from(pwd: &PathBuf) -> Result<Owners> {
        use std::fs;
        let mpath = pwd.join("teams.yml");
        trace!("Using teams file in {}", mpath.display());
        if !mpath.exists() {
            bail!("Teams file {} does not exist", mpath.display())
        }
        let data = fs::read_to_string(&mpath)?;
        let res = serde_yaml::from_str(&data)?;
        Ok(res)
    }
}

/// A set of slack channels
///
/// All channel ids must be slack guids (upper case starting with C)
/// In general, one must supply at least one support or internal channel.
///
/// If neither notifications or alerts have been specified, these will end up in
/// your internal or support channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSet {
    /// An internal slack channel for humans (no notifications)
    ///
    /// A place for the squad, tribe, or alliance. Internal.
    pub internal: Option<SlackChannel>,
    /// A slack channel for humans (no notifications)
    ///
    /// A place to ask humans about things
    pub support: Option<SlackChannel>,

    /// A slack channel for robots (non-urgent notifications)
    ///
    /// Upgrade notifications, pr requests, jira tickets, non-critical alerts.
    pub notifications: Option<SlackChannel>,
    /// A slack channel for robots (urgent notifications)
    ///
    /// Test failures in prod, production issues,
    pub alerts: Option<SlackChannel>,
}

/// A set of github teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubTeams {
    /// Team name on github in lowercase, dash-separated form
    pub team: String,
    /// Team on github with elevated permissions. Lowercase, dash-separated form.
    pub admins: Option<String>,
}

/// How service ownership is validated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceOwnership {
    /// A valid squad must be entered into metadata.team in manifests
    Squads,
    /// A valid squad or an old style deprecated teams entry
    SquadsOrLegacyTeam,
}

impl Default for ServiceOwnership {
    fn default() -> Self {
        Self::SquadsOrLegacyTeam
    }
}
