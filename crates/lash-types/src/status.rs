//! Task status types and checkbox character mappings

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{codes, LashError, Result};

/// Status of a task in the system
///
/// Tasks can be in one of five states, represented by different checkbox characters
/// in Markdown:
/// - `[ ]` - `Open` (not yet started)
/// - `[>]` - `InProgress` (actively being worked on)
/// - `[x]` - `Done` (completed successfully)
/// - `[-]` - `Waived` (marked as not applicable)
/// - `[!]` - `Blocked` (cannot proceed due to dependencies or other issues)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task not yet started
    #[default]
    Open,
    /// Task actively being worked on
    #[serde(rename = "in-progress")]
    InProgress,
    /// Task completed successfully
    Done,
    /// Task marked as not applicable
    Waived,
    /// Task cannot proceed (blocked by dependencies or other issues)
    Blocked,
}

impl TaskStatus {
    /// Convert status to checkbox character for Markdown representation
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::TaskStatus;
    ///
    /// assert_eq!(TaskStatus::Open.to_checkbox_char(), ' ');
    /// assert_eq!(TaskStatus::InProgress.to_checkbox_char(), '>');
    /// assert_eq!(TaskStatus::Done.to_checkbox_char(), 'x');
    /// assert_eq!(TaskStatus::Waived.to_checkbox_char(), '-');
    /// assert_eq!(TaskStatus::Blocked.to_checkbox_char(), '!');
    /// ```
    #[must_use]
    pub fn to_checkbox_char(self) -> char {
        match self {
            Self::Open => ' ',
            Self::InProgress => '>',
            Self::Done => 'x',
            Self::Waived => '-',
            Self::Blocked => '!',
        }
    }

    /// Parse status from checkbox character
    ///
    /// Accepts both uppercase and lowercase 'X' for done status.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::TaskStatus;
    ///
    /// assert_eq!(TaskStatus::from_checkbox_char(' ').unwrap(), TaskStatus::Open);
    /// assert_eq!(TaskStatus::from_checkbox_char('>').unwrap(), TaskStatus::InProgress);
    /// assert_eq!(TaskStatus::from_checkbox_char('x').unwrap(), TaskStatus::Done);
    /// assert_eq!(TaskStatus::from_checkbox_char('X').unwrap(), TaskStatus::Done);
    /// assert_eq!(TaskStatus::from_checkbox_char('-').unwrap(), TaskStatus::Waived);
    /// assert_eq!(TaskStatus::from_checkbox_char('!').unwrap(), TaskStatus::Blocked);
    /// assert!(TaskStatus::from_checkbox_char('?').is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `LashError::Parse` if the character is not a valid checkbox character.
    pub fn from_checkbox_char(c: char) -> Result<Self> {
        match c {
            ' ' => Ok(Self::Open),
            '>' => Ok(Self::InProgress),
            'x' | 'X' => Ok(Self::Done),
            '-' => Ok(Self::Waived),
            '!' => Ok(Self::Blocked),
            _ => Err(LashError::Parse {
                code: codes::E_PARSE_INVALID_CHECKBOX,
                message: format!("Invalid checkbox character: '{c}'"),
                location: None,
                snippet: Some(format!("[{c}]")),
                help: Some("checkboxes must be one of: [ ], [>], [x], [-], or [!]".to_string()),
            }),
        }
    }

    /// Check if the task is complete (either Done or Waived)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::TaskStatus;
    ///
    /// assert!(!TaskStatus::Open.is_complete());
    /// assert!(!TaskStatus::InProgress.is_complete());
    /// assert!(TaskStatus::Done.is_complete());
    /// assert!(TaskStatus::Waived.is_complete());
    /// assert!(!TaskStatus::Blocked.is_complete());
    /// ```
    #[must_use]
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Done | Self::Waived)
    }

    /// Check if this status requires dependencies to be checked
    ///
    /// Blocked tasks need dependency checking to determine if they can be unblocked.
    /// `Open` and `InProgress` tasks may need dependency checking for validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_types::TaskStatus;
    ///
    /// assert!(TaskStatus::Open.requires_dependencies());
    /// assert!(TaskStatus::InProgress.requires_dependencies());
    /// assert!(!TaskStatus::Done.requires_dependencies());
    /// assert!(!TaskStatus::Waived.requires_dependencies());
    /// assert!(TaskStatus::Blocked.requires_dependencies());
    /// ```
    #[must_use]
    pub fn requires_dependencies(self) -> bool {
        matches!(self, Self::Open | Self::InProgress | Self::Blocked)
    }

    /// Convert status to string representation
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in-progress",
            Self::Done => "done",
            Self::Waived => "waived",
            Self::Blocked => "blocked",
        }
    }

    /// Parse status from string (defensive, returns default for unknown values)
    ///
    /// Unlike `FromStr`, this returns the status directly without `Result`.
    /// Returns `Open` for invalid strings (defensive fallback).
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().replace('_', "-").as_str() {
            "in-progress" | "inprogress" => Self::InProgress,
            "done" => Self::Done,
            "waived" => Self::Waived,
            "blocked" => Self::Blocked,
            _ => Self::Open, // Default fallback for unknown or "open"
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Done => write!(f, "done"),
            Self::Waived => write!(f, "waived"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = LashError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "open" => Ok(Self::Open),
            "in-progress" | "inprogress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "waived" => Ok(Self::Waived),
            "blocked" => Ok(Self::Blocked),
            _ => Err(LashError::Parse {
                code: codes::E_LINT_STATUS_INCONSISTENCY,
                message: format!("Invalid status string: '{s}'"),
                location: None,
                snippet: Some(s.to_string()),
                help: Some(
                    "status must be one of: open, in-progress, done, waived, or blocked"
                        .to_string(),
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_checkbox_char() {
        assert_eq!(TaskStatus::Open.to_checkbox_char(), ' ');
        assert_eq!(TaskStatus::InProgress.to_checkbox_char(), '>');
        assert_eq!(TaskStatus::Done.to_checkbox_char(), 'x');
        assert_eq!(TaskStatus::Waived.to_checkbox_char(), '-');
        assert_eq!(TaskStatus::Blocked.to_checkbox_char(), '!');
    }

    #[test]
    fn test_from_checkbox_char() {
        assert_eq!(
            TaskStatus::from_checkbox_char(' ').unwrap(),
            TaskStatus::Open
        );
        assert_eq!(
            TaskStatus::from_checkbox_char('>').unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            TaskStatus::from_checkbox_char('x').unwrap(),
            TaskStatus::Done
        );
        assert_eq!(
            TaskStatus::from_checkbox_char('X').unwrap(),
            TaskStatus::Done
        );
        assert_eq!(
            TaskStatus::from_checkbox_char('-').unwrap(),
            TaskStatus::Waived
        );
        assert_eq!(
            TaskStatus::from_checkbox_char('!').unwrap(),
            TaskStatus::Blocked
        );
    }

    #[test]
    fn test_from_checkbox_char_error() {
        assert!(TaskStatus::from_checkbox_char('?').is_err());
        assert!(TaskStatus::from_checkbox_char('~').is_err());
        assert!(TaskStatus::from_checkbox_char('o').is_err());
    }

    #[test]
    fn test_checkbox_round_trip() {
        for status in &[
            TaskStatus::Open,
            TaskStatus::InProgress,
            TaskStatus::Done,
            TaskStatus::Waived,
            TaskStatus::Blocked,
        ] {
            let char = status.to_checkbox_char();
            let parsed = TaskStatus::from_checkbox_char(char).unwrap();
            assert_eq!(*status, parsed);
        }
    }

    #[test]
    fn test_from_str() {
        assert_eq!("open".parse::<TaskStatus>().unwrap(), TaskStatus::Open);
        assert_eq!(
            "in-progress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            "in_progress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            "inprogress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!("done".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
        assert_eq!("waived".parse::<TaskStatus>().unwrap(), TaskStatus::Waived);
        assert_eq!(
            "blocked".parse::<TaskStatus>().unwrap(),
            TaskStatus::Blocked
        );
    }

    #[test]
    fn test_from_str_case_insensitive() {
        assert_eq!("OPEN".parse::<TaskStatus>().unwrap(), TaskStatus::Open);
        assert_eq!(
            "IN-PROGRESS".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!(
            "In_Progress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!("Done".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
        assert_eq!("WAIVED".parse::<TaskStatus>().unwrap(), TaskStatus::Waived);
        assert_eq!(
            "BlOcKeD".parse::<TaskStatus>().unwrap(),
            TaskStatus::Blocked
        );
    }

    #[test]
    fn test_from_str_error() {
        assert!("invalid".parse::<TaskStatus>().is_err());
        assert!("pending".parse::<TaskStatus>().is_err());
        assert!("".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", TaskStatus::Open), "open");
        assert_eq!(format!("{}", TaskStatus::InProgress), "in-progress");
        assert_eq!(format!("{}", TaskStatus::Done), "done");
        assert_eq!(format!("{}", TaskStatus::Waived), "waived");
        assert_eq!(format!("{}", TaskStatus::Blocked), "blocked");
    }

    #[test]
    fn test_string_round_trip() {
        for status in &[
            TaskStatus::Open,
            TaskStatus::InProgress,
            TaskStatus::Done,
            TaskStatus::Waived,
            TaskStatus::Blocked,
        ] {
            let string = format!("{status}");
            let parsed: TaskStatus = string.parse().unwrap();
            assert_eq!(*status, parsed);
        }
    }

    #[test]
    fn test_is_complete() {
        assert!(!TaskStatus::Open.is_complete());
        assert!(!TaskStatus::InProgress.is_complete());
        assert!(TaskStatus::Done.is_complete());
        assert!(TaskStatus::Waived.is_complete());
        assert!(!TaskStatus::Blocked.is_complete());
    }

    #[test]
    fn test_requires_dependencies() {
        assert!(TaskStatus::Open.requires_dependencies());
        assert!(TaskStatus::InProgress.requires_dependencies());
        assert!(!TaskStatus::Done.requires_dependencies());
        assert!(!TaskStatus::Waived.requires_dependencies());
        assert!(TaskStatus::Blocked.requires_dependencies());
    }
}
