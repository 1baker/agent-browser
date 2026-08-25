use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Result of a policy check for an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    /// Action is allowed.
    Allow,
    /// Action is blocked with the given reason.
    Deny(String),
    /// Action requires confirmation before proceeding.
    RequiresConfirmation,
}

/// Stable consequence category used by agentic confirmation policy and
/// approval receipts. Categories describe what an action can change, not the
/// caller's claimed intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionConsequence {
    ReadOnly,
    Navigation,
    PageMutation,
    ExternalMutation,
    FileTransfer,
    Credentials,
    ScriptExecution,
    BrowserLifecycle,
    ControlPlane,
}

impl ActionConsequence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::Navigation => "navigation",
            Self::PageMutation => "page_mutation",
            Self::ExternalMutation => "external_mutation",
            Self::FileTransfer => "file_transfer",
            Self::Credentials => "credentials",
            Self::ScriptExecution => "script_execution",
            Self::BrowserLifecycle => "browser_lifecycle",
            Self::ControlPlane => "control_plane",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::ReadOnly => "Read browser or service state without changing the page.",
            Self::Navigation => "Change the selected page, history entry, or tab.",
            Self::PageMutation => "Change page-local input or document state.",
            Self::ExternalMutation => {
                "Interact with a page control that may commit an external effect."
            }
            Self::FileTransfer => "Move a file into or out of the browser session.",
            Self::Credentials => "Change authentication, cookie, storage, or permission state.",
            Self::ScriptExecution => "Execute caller-supplied code in the page context.",
            Self::BrowserLifecycle => "Change browser, target, attachment, or session lifecycle.",
            Self::ControlPlane => "Change or inspect agent-browser control-plane state.",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read_only" => Some(Self::ReadOnly),
            "navigation" => Some(Self::Navigation),
            "page_mutation" => Some(Self::PageMutation),
            "external_mutation" => Some(Self::ExternalMutation),
            "file_transfer" => Some(Self::FileTransfer),
            "credentials" => Some(Self::Credentials),
            "script_execution" => Some(Self::ScriptExecution),
            "browser_lifecycle" => Some(Self::BrowserLifecycle),
            "control_plane" => Some(Self::ControlPlane),
            _ => None,
        }
    }

    pub fn authority_rank(self) -> u8 {
        match self {
            Self::ReadOnly => 0,
            Self::Navigation => 1,
            Self::PageMutation => 2,
            Self::ExternalMutation => 3,
            Self::FileTransfer => 4,
            Self::Credentials => 5,
            Self::ScriptExecution => 6,
            Self::BrowserLifecycle => 7,
            Self::ControlPlane => 8,
        }
    }
}

pub fn action_consequence(action: &str) -> ActionConsequence {
    match action {
        "url" | "title" | "content" | "read_page" | "inspect" | "snapshot" | "screenshot"
        | "gettext" | "getattribute" | "gethtml" | "getstyles" | "count" | "getbox"
        | "isvisible" | "isenabled" | "ischecked" | "console" | "errors" | "requests"
        | "request_detail" | "cookies_get" | "storage_get" | "tab_list" | "browser_pid"
        | "wait" | "cdp_url" | "diagnostics" | "probe" | "stream_status" | "state_list"
        | "state_show" => ActionConsequence::ReadOnly,
        "task_authority_status" => ActionConsequence::ReadOnly,
        "task_authority_reconcile" => ActionConsequence::ControlPlane,
        "navigate" | "back" | "forward" | "reload" | "tab_new" | "tab_switch" => {
            ActionConsequence::Navigation
        }
        "fill" | "type" | "clear" | "focus" | "hover" | "scroll" | "scrollintoview"
        | "setcontent" | "viewport" | "useragent" | "media" | "timezone" | "locale"
        | "geolocation" | "headers" | "offline" => ActionConsequence::PageMutation,
        "click" | "dblclick" | "press" | "select" | "check" | "uncheck" | "ui_action"
        | "dialog" => ActionConsequence::ExternalMutation,
        "upload" | "download" | "wait_for_download" | "file_transfer" | "pdf" => {
            ActionConsequence::FileTransfer
        }
        "credentials_set" | "credentials_delete" | "auth_save" | "auth_delete" | "cookies_set"
        | "cookies_clear" | "storage_set" | "storage_clear" | "permissions" => {
            ActionConsequence::Credentials
        }
        "evaluate" => ActionConsequence::ScriptExecution,
        "launch"
        | "close"
        | "tab_close"
        | "cdp_attach"
        | "cdp_detach"
        | "cdp_free_launch"
        | "external_byop_adopt"
        | "runtime_handoff_prepare"
        | "runtime_handoff_resume"
        | "remote_view_open"
        | "view_takeover" => ActionConsequence::BrowserLifecycle,
        _ => ActionConsequence::ControlPlane,
    }
}

/// Policy configuration loaded from a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPolicy {
    #[serde(skip)]
    path: PathBuf,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    allow: Option<Vec<String>>,
    #[serde(default)]
    deny: Option<Vec<String>>,
    #[serde(default)]
    confirm: Option<Vec<String>>,
}

/// Confirmation categories parsed from AGENT_BROWSER_CONFIRM_ACTIONS.
#[derive(Debug, Clone)]
pub struct ConfirmActions {
    pub categories: HashSet<String>,
}

impl ConfirmActions {
    pub fn from_env() -> Option<Self> {
        let val = env::var("AGENT_BROWSER_CONFIRM_ACTIONS").ok()?;
        if val.is_empty() {
            return None;
        }
        let categories: HashSet<String> = val
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if categories.is_empty() {
            None
        } else {
            Some(Self { categories })
        }
    }

    pub fn requires_confirmation(&self, action: &str) -> bool {
        let consequence = action_consequence(action);
        self.categories.contains(action)
            || self.categories.contains(consequence.as_str())
            || (self.categories.contains("mutation")
                && matches!(
                    consequence,
                    ActionConsequence::PageMutation
                        | ActionConsequence::ExternalMutation
                        | ActionConsequence::FileTransfer
                        | ActionConsequence::Credentials
                        | ActionConsequence::ScriptExecution
                ))
    }
}

impl ActionPolicy {
    /// Load policy from a JSON file at the given path.
    pub fn load(path: &str) -> Result<Self, String> {
        let path_buf = PathBuf::from(path);
        let contents = fs::read_to_string(&path_buf)
            .map_err(|e| format!("Failed to read policy file: {}", e))?;
        let mut policy: ActionPolicy =
            serde_json::from_str(&contents).map_err(|e| format!("Invalid policy JSON: {}", e))?;
        policy.path = path_buf;
        Ok(policy)
    }

    /// Load policy if AGENT_BROWSER_ACTION_POLICY env var is set.
    /// Falls back to AGENT_BROWSER_POLICY for backwards compatibility.
    pub fn load_if_exists() -> Option<Self> {
        let path = env::var("AGENT_BROWSER_ACTION_POLICY")
            .or_else(|_| env::var("AGENT_BROWSER_POLICY"))
            .ok()?;
        Self::load(&path).ok()
    }

    /// Check whether an action is allowed, denied, or requires confirmation.
    pub fn check(&self, action: &str) -> PolicyResult {
        if let Some(deny) = &self.deny {
            if deny.iter().any(|a| a == action) {
                return PolicyResult::Deny(format!("Action '{}' is denied by policy", action));
            }
        }

        if let Some(confirm) = &self.confirm {
            if confirm.iter().any(|a| a == action) {
                return PolicyResult::RequiresConfirmation;
            }
        }

        if let Some(allow) = &self.allow {
            if !allow.iter().any(|a| a == action) {
                let is_default_allow = self
                    .default
                    .as_deref()
                    .map(|d| d.eq_ignore_ascii_case("allow"))
                    .unwrap_or(allow.is_empty());
                if !is_default_allow {
                    return PolicyResult::Deny(format!(
                        "Action '{}' is not in the allow list",
                        action
                    ));
                }
            }
        } else if let Some(ref default) = self.default {
            if default.eq_ignore_ascii_case("deny") {
                return PolicyResult::Deny(format!(
                    "Action '{}' denied: default policy is deny",
                    action
                ));
            }
        }

        PolicyResult::Allow
    }

    /// Reload policy from the file. Re-reads the JSON and updates the policy.
    pub fn reload(&mut self) -> Result<(), String> {
        let contents = fs::read_to_string(&self.path)
            .map_err(|e| format!("Failed to read policy file: {}", e))?;
        let mut policy: ActionPolicy =
            serde_json::from_str(&contents).map_err(|e| format!("Invalid policy JSON: {}", e))?;
        policy.path = self.path.clone();
        *self = policy;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;

    #[test]
    fn test_policy_allow_whitelist() {
        let json = r#"{"allow": ["click", "type"], "deny": [], "confirm": []}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("click"), PolicyResult::Allow);
        assert_eq!(policy.check("type"), PolicyResult::Allow);
        assert!(matches!(policy.check("navigate"), PolicyResult::Deny(_)));
    }

    #[test]
    fn test_policy_deny() {
        let json = r#"{"allow": [], "deny": ["delete"], "confirm": []}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert!(matches!(policy.check("delete"), PolicyResult::Deny(_)));
    }

    #[test]
    fn test_policy_confirm() {
        let json = r#"{"allow": [], "deny": [], "confirm": ["submit"]}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("submit"), PolicyResult::RequiresConfirmation);
    }

    #[test]
    fn test_policy_deny_takes_precedence() {
        let json = r#"{"allow": ["danger"], "deny": ["danger"], "confirm": []}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert!(matches!(policy.check("danger"), PolicyResult::Deny(_)));
    }

    #[test]
    fn test_policy_confirm_takes_precedence_over_allow() {
        let json = r#"{"allow": ["submit"], "deny": [], "confirm": ["submit"]}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("submit"), PolicyResult::RequiresConfirmation);
    }

    #[test]
    fn test_policy_empty_allow_allows_all() {
        let json = r#"{"allow": [], "deny": [], "confirm": []}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("anything"), PolicyResult::Allow);
    }

    #[test]
    fn test_policy_missing_allow_allows_all() {
        let json = r#"{"deny": []}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("anything"), PolicyResult::Allow);
    }

    #[test]
    fn test_policy_default_allow() {
        let json = r#"{"default": "allow", "deny": ["navigate"]}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("click"), PolicyResult::Allow);
        assert!(matches!(policy.check("navigate"), PolicyResult::Deny(_)));
    }

    #[test]
    fn test_policy_default_deny() {
        let json = r#"{"default": "deny", "allow": ["click"]}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.check("click"), PolicyResult::Allow);
        assert!(matches!(policy.check("navigate"), PolicyResult::Deny(_)));
    }

    #[test]
    fn test_policy_default_deny_with_empty_allow_denies_all() {
        let json = r#"{"default": "deny", "allow": []}"#;
        let policy: ActionPolicy = serde_json::from_str(json).unwrap();
        assert!(matches!(policy.check("navigate"), PolicyResult::Deny(_)));
    }

    #[test]
    fn test_confirm_actions_from_env() {
        let _guard = EnvGuard::new(&["AGENT_BROWSER_CONFIRM_ACTIONS"]);
        _guard.set("AGENT_BROWSER_CONFIRM_ACTIONS", "navigate,click,fill");
        let ca = ConfirmActions::from_env().unwrap();
        assert!(ca.requires_confirmation("navigate"));
        assert!(ca.requires_confirmation("click"));
        assert!(ca.requires_confirmation("fill"));
        assert!(!ca.requires_confirmation("screenshot"));
    }

    #[test]
    fn test_confirm_actions_accept_consequence_categories_and_mutation_group() {
        let category = ConfirmActions {
            categories: HashSet::from(["external_mutation".to_string()]),
        };
        assert!(category.requires_confirmation("click"));
        assert!(!category.requires_confirmation("snapshot"));

        let mutation = ConfirmActions {
            categories: HashSet::from(["mutation".to_string()]),
        };
        for action in ["fill", "press", "upload", "cookies_set", "evaluate"] {
            assert!(mutation.requires_confirmation(action), "{action}");
        }
        assert!(!mutation.requires_confirmation("title"));
        assert!(!mutation.requires_confirmation("navigate"));
    }

    #[test]
    fn test_action_consequence_classification_is_stable_for_agentic_boundaries() {
        assert_eq!(action_consequence("snapshot"), ActionConsequence::ReadOnly);
        assert_eq!(action_consequence("wait"), ActionConsequence::ReadOnly);
        assert_eq!(
            action_consequence("navigate"),
            ActionConsequence::Navigation
        );
        assert_eq!(action_consequence("fill"), ActionConsequence::PageMutation);
        assert_eq!(
            action_consequence("click"),
            ActionConsequence::ExternalMutation
        );
        assert_eq!(
            action_consequence("upload"),
            ActionConsequence::FileTransfer
        );
        assert_eq!(
            action_consequence("cookies_set"),
            ActionConsequence::Credentials
        );
        assert_eq!(
            action_consequence("evaluate"),
            ActionConsequence::ScriptExecution
        );
        assert_eq!(
            action_consequence("close"),
            ActionConsequence::BrowserLifecycle
        );
    }
}
