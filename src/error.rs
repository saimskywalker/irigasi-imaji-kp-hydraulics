//! Error type for every fallible operation in the crate.
//!
//! Library code in this crate never panics on bad input. Anything that can be
//! rejected is rejected through [`KpError`], with the offending parameter named
//! so that the caller can report it back to the engineer.

use std::error::Error;
use std::fmt;

/// Everything that can go wrong in an irrigation design calculation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum KpError {
    /// A parameter was `NaN` or infinite.
    NotFinite {
        /// Name of the offending parameter.
        parameter: &'static str,
        /// The value that was supplied.
        value: f64,
    },
    /// A parameter that must be strictly greater than zero was zero or negative.
    NotPositive {
        /// Name of the offending parameter.
        parameter: &'static str,
        /// The value that was supplied.
        value: f64,
    },
    /// A parameter that must be zero or greater was negative.
    Negative {
        /// Name of the offending parameter.
        parameter: &'static str,
        /// The value that was supplied.
        value: f64,
    },
    /// A parameter fell outside the range the method is valid over.
    OutOfRange {
        /// Name of the offending parameter.
        parameter: &'static str,
        /// The value that was supplied.
        value: f64,
        /// Lowest acceptable value, inclusive.
        min: f64,
        /// Highest acceptable value, inclusive.
        max: f64,
    },
    /// An iterative solver ran out of iterations before reaching its tolerance.
    ///
    /// The remaining bracket width or residual is reported so the caller can
    /// judge how far off the last iterate was.
    DidNotConverge {
        /// Name of the routine that gave up.
        routine: &'static str,
        /// Number of iterations performed.
        iterations: u32,
        /// Convergence tolerance that was requested.
        tolerance: f64,
        /// Width of the remaining bracket, or the residual, at the last iterate.
        residual: f64,
    },
    /// An iterative solver could not find a bracket containing the root.
    ///
    /// For a normal depth solve this normally means the requested discharge
    /// cannot be carried within the search limit, so the section is too small
    /// or the slope too flat.
    NoBracket {
        /// Name of the routine that failed to bracket.
        routine: &'static str,
        /// Upper limit of the search that was exhausted.
        search_limit: f64,
    },
    /// A caller-supplied lookup table was empty, so no value could be read off.
    EmptyTable {
        /// Name of the table.
        table: &'static str,
    },
    /// The requested argument is outside the range the supplied table covers.
    TableOutOfRange {
        /// Name of the table.
        table: &'static str,
        /// The argument that could not be looked up.
        argument: f64,
    },
}

impl fmt::Display for KpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KpError::NotFinite { parameter, value } => {
                write!(f, "{parameter} must be a finite number, got {value}")
            }
            KpError::NotPositive { parameter, value } => {
                write!(f, "{parameter} must be greater than zero, got {value}")
            }
            KpError::Negative { parameter, value } => {
                write!(f, "{parameter} must not be negative, got {value}")
            }
            KpError::OutOfRange {
                parameter,
                value,
                min,
                max,
            } => write!(
                f,
                "{parameter} must lie between {min} and {max} inclusive, got {value}"
            ),
            KpError::DidNotConverge {
                routine,
                iterations,
                tolerance,
                residual,
            } => write!(
                f,
                "{routine} did not converge to {tolerance} within {iterations} iterations, \
                 residual {residual}"
            ),
            KpError::NoBracket {
                routine,
                search_limit,
            } => write!(
                f,
                "{routine} could not bracket a solution below the search limit {search_limit}"
            ),
            KpError::EmptyTable { table } => write!(f, "lookup table {table} is empty"),
            KpError::TableOutOfRange { table, argument } => {
                write!(
                    f,
                    "lookup table {table} does not cover the argument {argument}"
                )
            }
        }
    }
}

impl Error for KpError {}

/// Shorthand for a result carrying a [`KpError`].
pub type Result<T> = std::result::Result<T, KpError>;

/// Rejects non-finite values.
pub(crate) fn finite(parameter: &'static str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(KpError::NotFinite { parameter, value })
    }
}

/// Rejects non-finite and negative values.
pub(crate) fn non_negative(parameter: &'static str, value: f64) -> Result<f64> {
    let value = finite(parameter, value)?;
    if value < 0.0 {
        Err(KpError::Negative { parameter, value })
    } else {
        Ok(value)
    }
}

/// Rejects non-finite, zero and negative values.
pub(crate) fn positive(parameter: &'static str, value: f64) -> Result<f64> {
    let value = finite(parameter, value)?;
    if value <= 0.0 {
        Err(KpError::NotPositive { parameter, value })
    } else {
        Ok(value)
    }
}

/// Rejects values outside an inclusive range.
pub(crate) fn in_range(parameter: &'static str, value: f64, min: f64, max: f64) -> Result<f64> {
    let value = finite(parameter, value)?;
    if value < min || value > max {
        Err(KpError::OutOfRange {
            parameter,
            value,
            min,
            max,
        })
    } else {
        Ok(value)
    }
}
