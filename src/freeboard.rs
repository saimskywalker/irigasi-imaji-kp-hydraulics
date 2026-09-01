//! Freeboard rules.
//!
//! Reference: KP-03, which gives canal freeboard (tinggi jagaan, the height of
//! the bank above the design water level) as a step function of design
//! discharge, read off a table.
//!
//! The table itself belongs to the criteria document and is not reproduced
//! here. Supply the rule the project is working to, either as a
//! [`FreeboardTable`] built from the values in the current KP-03 edition, or as
//! a [`FreeboardFn`] wrapping any function of discharge.
//!
//! [`FreeboardTable::default`] provides a small set of widely published
//! representative values so that an example or a first pass runs without a
//! table to hand. It is a convenience only. Confirm every value against the
//! current KP-03 edition and the project's own standards before using a design.
//!
//! # Units
//!
//! Discharge m³/s, freeboard m. See [`crate::units`].

use crate::error::{KpError, Result};
use crate::units::{Discharge, Length};

/// One band of a step freeboard table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeboardStep {
    /// Upper bound of the discharge band, inclusive, m³/s.
    pub up_to_discharge: Discharge,
    /// Freeboard that applies to the band, m.
    pub freeboard: Length,
}

impl FreeboardStep {
    /// Creates a band from plain figures in m³/s and m.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if either value is not finite or is negative.
    pub fn new(up_to_discharge: f64, freeboard: f64) -> Result<Self> {
        Ok(Self {
            up_to_discharge: Discharge::new(up_to_discharge)?,
            freeboard: Length::new(freeboard)?,
        })
    }
}

/// A rule giving the freeboard to apply at a design discharge.
pub trait FreeboardRule {
    /// Returns the freeboard for a design discharge, m.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the discharge falls outside what the rule covers.
    fn freeboard(&self, discharge: Discharge) -> Result<Length>;
}

/// A step table of freeboard against design discharge.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeboardTable {
    steps: Vec<FreeboardStep>,
    above_table: Option<Length>,
}

impl FreeboardTable {
    /// Builds a table from bands in ascending order of discharge.
    ///
    /// `above_table` is the freeboard for discharges beyond the last band.
    /// Passing `None` makes a discharge above the table an error rather than a
    /// silent extrapolation.
    ///
    /// # Errors
    ///
    /// - [`KpError::EmptyTable`] if there are no bands.
    /// - [`KpError::OutOfRange`] if the bands are not in ascending order of
    ///   discharge.
    pub fn new(steps: Vec<FreeboardStep>, above_table: Option<Length>) -> Result<Self> {
        if steps.is_empty() {
            return Err(KpError::EmptyTable {
                table: "FreeboardTable",
            });
        }
        for pair in steps.windows(2) {
            if pair[1].up_to_discharge.value() <= pair[0].up_to_discharge.value() {
                return Err(KpError::OutOfRange {
                    parameter: "FreeboardTable band upper bound",
                    value: pair[1].up_to_discharge.value(),
                    min: pair[0].up_to_discharge.value(),
                    max: f64::MAX,
                });
            }
        }
        Ok(Self { steps, above_table })
    }

    /// Returns the bands of the table.
    pub fn steps(&self) -> &[FreeboardStep] {
        &self.steps
    }
}

impl Default for FreeboardTable {
    /// A convenience default with a handful of widely published representative
    /// canal freeboard values, in m, banded by design discharge in m³/s:
    /// below 0.5, then to 1.5, 5, 10 and 15, and above that.
    ///
    /// These are representative figures for an earth irrigation canal, provided
    /// so that examples and first passes run without a table to hand. They are
    /// not a reproduction of the criteria and they are not authoritative.
    /// Confirm them against the current KP-03 edition before using a design.
    fn default() -> Self {
        let band = |q: f64, f: f64| FreeboardStep {
            up_to_discharge: Discharge::from_valid(q),
            freeboard: Length::from_valid(f),
        };
        Self {
            steps: vec![
                band(0.5, 0.40),
                band(1.5, 0.50),
                band(5.0, 0.60),
                band(10.0, 0.75),
                band(15.0, 0.85),
            ],
            above_table: Some(Length::from_valid(1.00)),
        }
    }
}

impl FreeboardRule for FreeboardTable {
    fn freeboard(&self, discharge: Discharge) -> Result<Length> {
        for step in &self.steps {
            if discharge.value() <= step.up_to_discharge.value() {
                return Ok(step.freeboard);
            }
        }
        self.above_table.ok_or(KpError::TableOutOfRange {
            table: "FreeboardTable",
            argument: discharge.value(),
        })
    }
}

/// Wraps any function of discharge as a freeboard rule.
///
/// # Examples
///
/// ```
/// use kp_irigasi::freeboard::{FreeboardFn, FreeboardRule};
/// use kp_irigasi::units::{Discharge, Length};
///
/// // A project rule of one third of the flow depth is not expressible as a
/// // function of discharge alone, but a continuous curve on discharge is.
/// let rule = FreeboardFn::new(|q: Discharge| Length::new(0.35 + 0.05 * q.value().sqrt()));
/// let f = rule.freeboard(Discharge::new(4.0)?)?;
/// assert!((f.value() - 0.45).abs() < 1e-12);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub struct FreeboardFn<F>
where
    F: Fn(Discharge) -> Result<Length>,
{
    rule: F,
}

impl<F> FreeboardFn<F>
where
    F: Fn(Discharge) -> Result<Length>,
{
    /// Wraps the given function.
    pub const fn new(rule: F) -> Self {
        Self { rule }
    }
}

impl<F> FreeboardRule for FreeboardFn<F>
where
    F: Fn(Discharge) -> Result<Length>,
{
    fn freeboard(&self, discharge: Discharge) -> Result<Length> {
        (self.rule)(discharge)
    }
}

/// A single freeboard applied at any discharge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedFreeboard(pub Length);

impl FreeboardRule for FixedFreeboard {
    fn freeboard(&self, _discharge: Discharge) -> Result<Length> {
        Ok(self.0)
    }
}
