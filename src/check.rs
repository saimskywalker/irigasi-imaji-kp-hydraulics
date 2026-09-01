//! Design checks carried alongside every design result.
//!
//! A design routine in this crate never reduces its answer to a single number.
//! It returns the geometry, the intermediate values, and a list of [`Check`]
//! items recording every criterion that was tested, the value that was tested,
//! the limit it was tested against, and whether it passed. That is what an
//! engineer needs in order to verify the result by hand and to record the check
//! in a design note.
//!
//! Units follow the crate convention, see [`crate::units`]. Each check carries
//! its own unit label as text because the checks are heterogeneous.

use std::fmt;

/// Outcome of a single design check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// The criterion is satisfied.
    Pass,
    /// The criterion is not satisfied. The design result is still returned, so
    /// the engineer can see by how much it fails and decide what to change.
    Fail,
    /// The criterion was not tested, normally because the caller supplied no
    /// limit for it.
    NotChecked,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            CheckStatus::Pass => "PASS",
            CheckStatus::Fail => "FAIL",
            CheckStatus::NotChecked => "NOT CHECKED",
        };
        f.write_str(text)
    }
}

/// A single named design check with the value tested and the limit applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// Short name of the criterion, for example `"maximum velocity"`.
    pub name: &'static str,
    /// The value that was tested, in the unit given by [`Check::unit`].
    pub value: f64,
    /// Unit label for [`Check::value`], for example `"m/s"`.
    pub unit: &'static str,
    /// The limit the value was tested against, where there is one.
    pub limit: Option<f64>,
    /// Text describing the criterion, including the KP part it comes from where
    /// the caller has supplied that context.
    pub criterion: String,
    /// Whether the criterion is satisfied.
    pub status: CheckStatus,
}

impl Check {
    /// Builds a check for a value that must not exceed an upper limit.
    ///
    /// Passing `None` as the limit records the check as
    /// [`CheckStatus::NotChecked`], which is how an optional criterion the
    /// caller chose not to apply is reported.
    pub fn at_most(
        name: &'static str,
        value: f64,
        unit: &'static str,
        limit: Option<f64>,
        criterion: impl Into<String>,
    ) -> Self {
        let status = match limit {
            Some(limit) if value <= limit => CheckStatus::Pass,
            Some(_) => CheckStatus::Fail,
            None => CheckStatus::NotChecked,
        };
        Self {
            name,
            value,
            unit,
            limit,
            criterion: criterion.into(),
            status,
        }
    }

    /// Builds a check for a value that must reach a lower limit.
    ///
    /// Passing `None` as the limit records the check as
    /// [`CheckStatus::NotChecked`].
    pub fn at_least(
        name: &'static str,
        value: f64,
        unit: &'static str,
        limit: Option<f64>,
        criterion: impl Into<String>,
    ) -> Self {
        let status = match limit {
            Some(limit) if value >= limit => CheckStatus::Pass,
            Some(_) => CheckStatus::Fail,
            None => CheckStatus::NotChecked,
        };
        Self {
            name,
            value,
            unit,
            limit,
            criterion: criterion.into(),
            status,
        }
    }

    /// Builds a check whose outcome the caller has already determined.
    pub fn verdict(
        name: &'static str,
        value: f64,
        unit: &'static str,
        status: CheckStatus,
        criterion: impl Into<String>,
    ) -> Self {
        Self {
            name,
            value,
            unit,
            limit: None,
            criterion: criterion.into(),
            status,
        }
    }

    /// Returns true when the check is not a failure, that is it passed or was
    /// not applied.
    pub fn is_acceptable(&self) -> bool {
        self.status != CheckStatus::Fail
    }
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<28} {:>10.3} {:<8}", self.name, self.value, self.unit)?;
        match self.limit {
            Some(limit) => write!(f, " limit {limit:>8.3} ")?,
            None => write!(f, " {:<15}", "")?,
        }
        write!(f, " {:<11} {}", self.status.to_string(), self.criterion)
    }
}

/// Convenience over a list of checks.
pub trait CheckList {
    /// Returns true when no check in the list has failed.
    fn all_pass(&self) -> bool;
    /// Returns the failed checks.
    fn failures(&self) -> Vec<&Check>;
}

impl CheckList for [Check] {
    fn all_pass(&self) -> bool {
        self.iter().all(Check::is_acceptable)
    }

    fn failures(&self) -> Vec<&Check> {
        self.iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .collect()
    }
}
