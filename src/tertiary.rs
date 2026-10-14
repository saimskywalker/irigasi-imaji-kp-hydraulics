//! Tertiary block sizing and layout.
//!
//! Reference: KP-01 and KP-07, petak tersier (tertiary block, the unit of land
//! served by one tertiary offtake from a secondary canal) and petak kuarter
//! (quaternary block, the sub unit served by one quaternary channel).
//!
//! A tertiary block is served through a saluran tersier (tertiary channel) from
//! a bangunan sadap (offtake structure) on the secondary canal. It is divided
//! into quaternary blocks, each served by a saluran kuarter (quaternary
//! channel). The criteria set limits on block area and on channel length, so
//! that water reaches the far end without excessive loss and so that the
//! farmers' water users association can operate the block.
//!
//! The limits themselves are read off the criteria and are supplied as
//! [`TertiaryCriteria`]. The [`Default`] implementation carries a handful of
//! widely published representative figures, as a convenience for examples, and
//! is not authoritative.
//!
//! # Units
//!
//! Areas ha, channel lengths m. See [`crate::units`].
//!
//! # Disclaimer
//!
//! A calculation aid, not a substitute for a licensed engineer's judgement.
//! Block layout is decided on the ground, against topography, land ownership
//! and the existing tertiary network, not by arithmetic alone.

use std::fmt;

use crate::check::{Check, CheckList};
use crate::error::{positive, Result};
use crate::units::{guard_result, Hectares, Length};

/// Limits applied to a tertiary block, all supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TertiaryCriteria {
    /// Typical or target tertiary block area, ha.
    pub typical_area: Hectares,
    /// Maximum tertiary block area, ha.
    pub maximum_area: Hectares,
    /// Maximum quaternary block area, ha.
    pub maximum_quaternary_area: Hectares,
    /// Maximum length of the tertiary channel, m.
    pub maximum_tertiary_channel_length: Length,
    /// Maximum length of a quaternary channel, m.
    pub maximum_quaternary_channel_length: Length,
}

impl Default for TertiaryCriteria {
    /// A convenience default of 50 ha typical and 150 ha maximum tertiary block,
    /// 15 ha maximum quaternary block, 1 500 m maximum tertiary channel and
    /// 500 m maximum quaternary channel.
    ///
    /// These are widely published representative figures for lowland rice on
    /// flat land, provided so that examples run without the criteria to hand.
    /// They are not a reproduction of the criteria and they are not
    /// authoritative. Confirm every figure against the current KP-01 and KP-07
    /// editions, and note that the figures for undulating and hilly land are
    /// different.
    fn default() -> Self {
        Self {
            typical_area: Hectares::from_valid(50.0),
            maximum_area: Hectares::from_valid(150.0),
            maximum_quaternary_area: Hectares::from_valid(15.0),
            maximum_tertiary_channel_length: Length::from_valid(1500.0),
            maximum_quaternary_channel_length: Length::from_valid(500.0),
        }
    }
}

/// The result of assessing one tertiary block against the criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct TertiaryBlock {
    /// Gross area of the block, ha.
    pub area: Hectares,
    /// Length of the tertiary channel serving the block, m.
    pub tertiary_channel_length: Length,
    /// Length of the longest quaternary channel in the block, m.
    pub quaternary_channel_length: Length,
    /// Number of quaternary blocks needed so that none exceeds the maximum
    /// quaternary area, rounded up.
    pub quaternary_blocks: u32,
    /// Mean quaternary block area implied by that count, ha.
    pub mean_quaternary_area: Hectares,
    /// The criteria applied.
    pub criteria: TertiaryCriteria,
    /// Every check, passed or failed.
    pub checks: Vec<Check>,
}

impl TertiaryBlock {
    /// Returns true when no check failed.
    pub fn all_checks_pass(&self) -> bool {
        self.checks.all_pass()
    }
}

impl fmt::Display for TertiaryBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Tertiary block (petak tersier) assessment, KP-01 and KP-07"
        )?;
        writeln!(
            f,
            "  block area                  {:>10.1} ha",
            self.area.value()
        )?;
        writeln!(
            f,
            "  tertiary channel length     {:>10.0} m",
            self.tertiary_channel_length.value()
        )?;
        writeln!(
            f,
            "  longest quaternary channel  {:>10.0} m",
            self.quaternary_channel_length.value()
        )?;
        writeln!(
            f,
            "  quaternary blocks required  {:>10}",
            self.quaternary_blocks
        )?;
        writeln!(
            f,
            "  mean quaternary block area  {:>10.2} ha",
            self.mean_quaternary_area.value()
        )?;
        writeln!(f, "Checks")?;
        for check in &self.checks {
            writeln!(f, "  {check}")?;
        }
        write!(
            f,
            "Overall: {}",
            if self.all_checks_pass() {
                "all checks pass"
            } else {
                "one or more checks FAIL, see above"
            }
        )
    }
}

/// Assesses a tertiary block against the layout criteria and works out how many
/// quaternary blocks it needs.
///
/// The quaternary block count is the block area divided by the maximum
/// quaternary area, rounded up. That is a first cut for a layout study: the
/// real division follows field boundaries and the existing channel network.
///
/// # Errors
///
/// Returns [`crate::KpError::NotPositive`] if the block area or the maximum
/// quaternary area is zero.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::tertiary::{assess_tertiary_block, TertiaryCriteria};
/// use imaji_kp_hydraulics::units::{Hectares, Length};
///
/// // 96 ha block, tertiary channel 1 200 m, longest quaternary channel 420 m.
/// // Quaternary blocks: ceil(96 / 15) = 7, mean area 96 / 7 = 13.71 ha.
/// let block = assess_tertiary_block(
///     Hectares::new(96.0)?,
///     Length::new(1200.0)?,
///     Length::new(420.0)?,
///     TertiaryCriteria::default(),
/// )?;
/// assert_eq!(block.quaternary_blocks, 7);
/// assert!((block.mean_quaternary_area.value() - 13.714_29).abs() < 1e-4);
/// assert!(block.all_checks_pass());
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn assess_tertiary_block(
    area: Hectares,
    tertiary_channel_length: Length,
    quaternary_channel_length: Length,
    criteria: TertiaryCriteria,
) -> Result<TertiaryBlock> {
    let area_value = positive("area", area.value())?;
    let max_quaternary = positive(
        "maximum_quaternary_area",
        criteria.maximum_quaternary_area.value(),
    )?;

    let quaternary_blocks = count_blocks(area_value, max_quaternary)?;
    let mean_quaternary_area = Hectares::new(guard_result(
        "mean_quaternary_area",
        area_value / f64::from(quaternary_blocks),
    )?)?;

    let checks = vec![
        Check::at_most(
            "tertiary block area",
            area_value,
            "ha",
            Some(criteria.maximum_area.value()),
            "KP-01 maximum petak tersier area, limit supplied by the caller",
        ),
        Check::at_most(
            "tertiary channel length",
            tertiary_channel_length.value(),
            "m",
            Some(criteria.maximum_tertiary_channel_length.value()),
            "KP-07 maximum saluran tersier length, limit supplied by the caller",
        ),
        Check::at_most(
            "quaternary channel length",
            quaternary_channel_length.value(),
            "m",
            Some(criteria.maximum_quaternary_channel_length.value()),
            "KP-07 maximum saluran kuarter length, limit supplied by the caller",
        ),
        Check::at_most(
            "mean quaternary area",
            mean_quaternary_area.value(),
            "ha",
            Some(max_quaternary),
            "KP-01 maximum petak kuarter area, limit supplied by the caller",
        ),
    ];

    Ok(TertiaryBlock {
        area,
        tertiary_channel_length,
        quaternary_channel_length,
        quaternary_blocks,
        mean_quaternary_area,
        criteria,
        checks,
    })
}

/// Number of tertiary blocks a secondary canal must serve for a command area.
///
/// The count is the command area divided by the block area, rounded up. Use the
/// typical block area rather than the maximum, so that the layout has room to
/// follow the ground.
///
/// # Errors
///
/// Returns [`crate::KpError::NotPositive`] if either area is zero.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::tertiary::blocks_in_command_area;
/// use imaji_kp_hydraulics::units::Hectares;
///
/// // 640 ha served by blocks of 50 ha: ceil(640 / 50) = 13 blocks.
/// let n = blocks_in_command_area(Hectares::new(640.0)?, Hectares::new(50.0)?)?;
/// assert_eq!(n, 13);
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn blocks_in_command_area(command_area: Hectares, block_area: Hectares) -> Result<u32> {
    let command = positive("command_area", command_area.value())?;
    let block = positive("block_area", block_area.value())?;
    count_blocks(command, block)
}

fn count_blocks(total: f64, per_block: f64) -> Result<u32> {
    let count = guard_result("block_count", (total / per_block).ceil())?;
    if count > f64::from(u32::MAX) {
        return Err(crate::KpError::OutOfRange {
            parameter: "block_count",
            value: count,
            min: 1.0,
            max: f64::from(u32::MAX),
        });
    }
    Ok(count.max(1.0) as u32)
}
