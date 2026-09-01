//! Culvert and siphon sizing, head loss for a barrel running full.
//!
//! Reference: KP-04, gorong gorong (culvert) and siphon (inverted siphon). The
//! head the structure needs is the sum of an entry loss, the friction loss
//! along the barrel, any bend losses, and the exit loss:
//!
//! ```text
//! hv = V² / 2g                              velocity head
//! h_entry    = k_entry × hv
//! h_bend     = sum(k_bend) × hv
//! h_exit     = k_exit × hv
//! h_friction = L (n V / R^(2/3))²           Manning form
//! H = h_entry + h_bend + h_exit + h_friction
//! ```
//!
//! The entry and exit loss coefficients depend on the inlet and outlet
//! geometry, and KP-04 gives them for the standard transitions. They are caller
//! input here.
//!
//! All relations assume the barrel runs full, which is the design condition for
//! a siphon and for a culvert under a road crossing at design flow. A part full
//! culvert is an open channel problem: use [`crate::uniform_flow`] instead.
//!
//! # Units
//!
//! Discharge m³/s, lengths m, velocity m/s, Manning n in s·m^(-1/3). See
//! [`crate::units`].
//!
//! # Disclaimer
//!
//! Every routine here is a calculation aid, not a substitute for a licensed
//! engineer's judgement. A real culvert or siphon also needs checks on
//! sedimentation, on air entrainment, on structural loading and on the
//! transitions, which this module does not perform.

use std::fmt;

use crate::check::{Check, CheckList};
use crate::error::{non_negative, positive, KpError, Result};
use crate::uniform_flow::VelocityLimits;
use crate::units::{guard_result, Area, Discharge, Length, ManningN, Velocity};
use crate::GRAVITY;

/// A barrel cross section running full.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Barrel {
    /// A circular pipe.
    Circular {
        /// Internal diameter, m.
        diameter: Length,
    },
    /// A rectangular box.
    Rectangular {
        /// Internal width, m.
        width: Length,
        /// Internal height, m.
        height: Length,
    },
}

impl Barrel {
    /// Full bore flow area, m².
    ///
    /// # Errors
    ///
    /// Returns [`KpError::NotPositive`] if a dimension is zero.
    pub fn flow_area(&self) -> Result<Area> {
        match *self {
            Barrel::Circular { diameter } => {
                let d = positive("diameter", diameter.value())?;
                Area::new(std::f64::consts::PI * d * d / 4.0)
            }
            Barrel::Rectangular { width, height } => {
                let w = positive("width", width.value())?;
                let h = positive("height", height.value())?;
                Area::new(w * h)
            }
        }
    }

    /// Wetted perimeter when running full, m.
    ///
    /// # Errors
    ///
    /// Returns [`KpError::NotPositive`] if a dimension is zero.
    pub fn wetted_perimeter(&self) -> Result<Length> {
        match *self {
            Barrel::Circular { diameter } => {
                let d = positive("diameter", diameter.value())?;
                Length::new(std::f64::consts::PI * d)
            }
            Barrel::Rectangular { width, height } => {
                let w = positive("width", width.value())?;
                let h = positive("height", height.value())?;
                Length::new(2.0 * (w + h))
            }
        }
    }

    /// Hydraulic radius when running full, m. For a circular barrel this is
    /// exactly `D/4`.
    ///
    /// # Errors
    ///
    /// Returns [`KpError::NotPositive`] if a dimension is zero.
    pub fn hydraulic_radius(&self) -> Result<Length> {
        let area = self.flow_area()?;
        let perimeter = positive("wetted_perimeter", self.wetted_perimeter()?.value())?;
        Length::new(area.value() / perimeter)
    }
}

impl fmt::Display for Barrel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Barrel::Circular { diameter } => write!(f, "circular D = {:.3} m", diameter.value()),
            Barrel::Rectangular { width, height } => {
                write!(
                    f,
                    "rectangular {:.3} m × {:.3} m",
                    width.value(),
                    height.value()
                )
            }
        }
    }
}

/// Local loss coefficients for a barrel, all as multiples of the velocity head.
///
/// KP-04 gives entry and exit values for the standard inlet and outlet
/// transitions. Bend losses are the sum of the coefficients of every bend in
/// the barrel, which for a siphon means both the descending and the ascending
/// elbow.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LossCoefficients {
    /// Entry loss coefficient.
    pub entry: f64,
    /// Exit loss coefficient.
    pub exit: f64,
    /// Sum of the bend loss coefficients.
    pub bends: f64,
}

impl LossCoefficients {
    /// Creates a set of coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`KpError::Negative`] if any coefficient is negative.
    pub fn new(entry: f64, exit: f64, bends: f64) -> Result<Self> {
        Ok(Self {
            entry: non_negative("entry", entry)?,
            exit: non_negative("exit", exit)?,
            bends: non_negative("bends", bends)?,
        })
    }

    /// Sum of the three coefficients.
    pub fn total(&self) -> f64 {
        self.entry + self.exit + self.bends
    }
}

/// Head loss through a barrel running full, broken into its terms.
#[derive(Debug, Clone, PartialEq)]
pub struct ConduitHeadLoss {
    /// The barrel.
    pub barrel: Barrel,
    /// Discharge, m³/s.
    pub discharge: Discharge,
    /// Barrel length, m.
    pub length: Length,
    /// Full bore flow area, m².
    pub flow_area: Area,
    /// Hydraulic radius, m.
    pub hydraulic_radius: Length,
    /// Mean velocity in the barrel, m/s.
    pub velocity: Velocity,
    /// Velocity head V²/2g, m.
    pub velocity_head: Length,
    /// Entry loss, m.
    pub entry_loss: Length,
    /// Bend losses, m.
    pub bend_loss: Length,
    /// Exit loss, m.
    pub exit_loss: Length,
    /// Friction loss along the barrel, m.
    pub friction_loss: Length,
    /// Total head loss, m. This is the head the structure consumes, that is the
    /// drop in water level from upstream to downstream.
    pub total_head_loss: Length,
    /// Velocity checks against the caller-supplied limits.
    pub checks: Vec<Check>,
}

impl ConduitHeadLoss {
    /// Returns true when no check failed.
    pub fn all_checks_pass(&self) -> bool {
        self.checks.all_pass()
    }
}

impl fmt::Display for ConduitHeadLoss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Culvert or siphon head loss, KP-04, barrel running full")?;
        writeln!(f, "  barrel                 {}", self.barrel)?;
        writeln!(
            f,
            "  discharge Q            {:>9.4} m³/s",
            self.discharge.value()
        )?;
        writeln!(f, "  barrel length L        {:>9.2} m", self.length.value())?;
        writeln!(
            f,
            "  flow area A            {:>9.4} m²",
            self.flow_area.value()
        )?;
        writeln!(
            f,
            "  hydraulic radius R     {:>9.4} m",
            self.hydraulic_radius.value()
        )?;
        writeln!(
            f,
            "  velocity V             {:>9.3} m/s",
            self.velocity.value()
        )?;
        writeln!(
            f,
            "  velocity head V²/2g    {:>9.4} m",
            self.velocity_head.value()
        )?;
        writeln!(
            f,
            "  entry loss             {:>9.4} m",
            self.entry_loss.value()
        )?;
        writeln!(
            f,
            "  bend loss              {:>9.4} m",
            self.bend_loss.value()
        )?;
        writeln!(
            f,
            "  friction loss          {:>9.4} m",
            self.friction_loss.value()
        )?;
        writeln!(
            f,
            "  exit loss              {:>9.4} m",
            self.exit_loss.value()
        )?;
        writeln!(
            f,
            "  total head loss H      {:>9.4} m",
            self.total_head_loss.value()
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

/// What the caller supplies to [`conduit_head_loss`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConduitInput {
    /// The barrel.
    pub barrel: Barrel,
    /// Design discharge, m³/s.
    pub discharge: Discharge,
    /// Barrel length along the axis, m.
    pub length: Length,
    /// Manning roughness of the barrel lining, s·m^(-1/3).
    pub roughness: ManningN,
    /// Local loss coefficients.
    pub coefficients: LossCoefficients,
    /// Permissible velocity range in the barrel. KP-04 keeps a siphon velocity
    /// high enough to move silt and low enough to limit head loss and scour at
    /// the outlet; the values are read from the criteria and supplied here.
    pub velocity_limits: VelocityLimits,
}

/// Computes the head loss through a culvert or siphon barrel running full.
///
/// # Errors
///
/// Returns [`KpError`] if a barrel dimension, the discharge, the length or the
/// roughness is zero.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// Check the result against the governing criteria.
///
/// # Examples
///
/// ```
/// use kp_irigasi::conduit::{conduit_head_loss, Barrel, ConduitInput, LossCoefficients};
/// use kp_irigasi::uniform_flow::VelocityLimits;
/// use kp_irigasi::units::{Discharge, Length, ManningN};
///
/// // D = 0.80 m, Q = 0.60 m³/s, L = 24 m, n = 0.014, k_in = 0.5, k_out = 1.0
/// // A = pi × 0.64 / 4 = 0.502655 m², V = 0.60 / 0.502655 = 1.19366 m/s
/// // hv = 1.193662² / 19.62 = 0.0726213 m
/// // R = D/4 = 0.20 m, R^(2/3) = 0.3419952
/// // hf = 24 × (0.014 × 1.193662 / 0.3419952)² = 24 × 0.00238769 = 0.0573047 m
/// // H = 1.5 × 0.0726213 + 0.0573047 = 0.1089319 + 0.0573047 = 0.1662366 m
/// let loss = conduit_head_loss(&ConduitInput {
///     barrel: Barrel::Circular { diameter: Length::new(0.80)? },
///     discharge: Discharge::new(0.60)?,
///     length: Length::new(24.0)?,
///     roughness: ManningN::new(0.014)?,
///     coefficients: LossCoefficients::new(0.5, 1.0, 0.0)?,
///     velocity_limits: VelocityLimits::from_range(0.6, 3.0)?,
/// })?;
/// assert!((loss.total_head_loss.value() - 0.1662366).abs() < 1e-7);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn conduit_head_loss(input: &ConduitInput) -> Result<ConduitHeadLoss> {
    let q = positive("discharge", input.discharge.value())?;
    let length = positive("length", input.length.value())?;
    let n = positive("roughness", input.roughness.value())?;

    let area = input.barrel.flow_area()?;
    let radius = positive("hydraulic_radius", input.barrel.hydraulic_radius()?.value())?;
    let velocity = guard_result("velocity", q / area.value())?;
    let velocity_head = velocity * velocity / (2.0 * GRAVITY);

    let entry_loss = input.coefficients.entry * velocity_head;
    let bend_loss = input.coefficients.bends * velocity_head;
    let exit_loss = input.coefficients.exit * velocity_head;
    let friction_slope = {
        let s = n * velocity / radius.powf(2.0 / 3.0);
        s * s
    };
    let friction_loss = guard_result("friction_loss", friction_slope * length)?;
    let total = entry_loss + bend_loss + exit_loss + friction_loss;

    let velocity = Velocity::new(velocity)?;
    let checks = input.velocity_limits.check(velocity);

    Ok(ConduitHeadLoss {
        barrel: input.barrel,
        discharge: input.discharge,
        length: input.length,
        flow_area: area,
        hydraulic_radius: Length::new(radius)?,
        velocity,
        velocity_head: Length::new(velocity_head)?,
        entry_loss: Length::new(entry_loss)?,
        bend_loss: Length::new(bend_loss)?,
        exit_loss: Length::new(exit_loss)?,
        friction_loss: Length::new(friction_loss)?,
        total_head_loss: Length::new(total)?,
        checks,
    })
}

/// Settings for the barrel sizing solver.
///
/// The solver is a bisection on diameter. Head loss falls monotonically as the
/// diameter grows, so a bracket is safe to find and bisection cannot diverge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarrelSizingOptions {
    /// Convergence tolerance on the diameter, m. Default 1e-4 m.
    pub tolerance: f64,
    /// Maximum bisection steps. Default 100.
    pub max_iterations: u32,
    /// Smallest diameter to consider, m. Default 0.10 m.
    pub min_diameter: f64,
    /// Largest diameter to consider, m. Default 5.0 m. If the head loss at this
    /// diameter still exceeds the allowance, the solver reports
    /// [`KpError::NoBracket`].
    pub max_diameter: f64,
}

impl Default for BarrelSizingOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-4,
            max_iterations: 100,
            min_diameter: 0.10,
            max_diameter: 5.0,
        }
    }
}

/// The smallest circular barrel that keeps the head loss within an allowance.
#[derive(Debug, Clone, PartialEq)]
pub struct BarrelSizing {
    /// The required diameter, m, at the solver tolerance.
    pub diameter: Length,
    /// The head loss at that diameter, with all its terms.
    pub head_loss: ConduitHeadLoss,
    /// The allowable head loss the barrel was sized against, m.
    pub allowable_head_loss: Length,
    /// Number of bisection steps performed.
    pub iterations: u32,
}

/// Sizes a circular culvert or siphon barrel for an allowable head loss.
///
/// Bisects on diameter until the bracket is narrower than the tolerance, then
/// returns the upper end of the bracket, which is the side that satisfies the
/// allowance. The velocity checks in the returned head loss come from the
/// caller-supplied limits and may fail: a barrel large enough to keep the head
/// loss down can be too large to keep silt moving, and that conflict is
/// reported rather than resolved.
///
/// # Errors
///
/// - [`KpError::NoBracket`] if even the largest diameter considered cannot meet
///   the allowance.
/// - [`KpError::DidNotConverge`] if the iteration limit is reached first.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// The diameter must then be rounded up to a manufactured size and rechecked.
///
/// # Examples
///
/// ```
/// use kp_irigasi::conduit::{size_circular_barrel, BarrelSizingOptions, LossCoefficients};
/// use kp_irigasi::uniform_flow::VelocityLimits;
/// use kp_irigasi::units::{Discharge, Length, ManningN};
///
/// let sizing = size_circular_barrel(
///     Discharge::new(0.60)?,
///     Length::new(24.0)?,
///     ManningN::new(0.014)?,
///     LossCoefficients::new(0.5, 1.0, 0.0)?,
///     Length::new(0.20)?,
///     VelocityLimits::from_range(0.6, 3.0)?,
///     BarrelSizingOptions::default(),
/// )?;
/// // The 0.80 m barrel of the head loss example needs 0.166 m, so an
/// // allowance of 0.20 m is met by something a little smaller.
/// assert!(sizing.diameter.value() < 0.80);
/// assert!(sizing.head_loss.total_head_loss.value() <= 0.20);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
#[allow(clippy::too_many_arguments)]
pub fn size_circular_barrel(
    discharge: Discharge,
    length: Length,
    roughness: ManningN,
    coefficients: LossCoefficients,
    allowable_head_loss: Length,
    velocity_limits: VelocityLimits,
    options: BarrelSizingOptions,
) -> Result<BarrelSizing> {
    const ROUTINE: &str = "size_circular_barrel";

    let allowance = positive("allowable_head_loss", allowable_head_loss.value())?;
    let tolerance = positive("tolerance", options.tolerance)?;
    let mut low = positive("min_diameter", options.min_diameter)?;
    let mut high = positive("max_diameter", options.max_diameter)?;
    if high <= low {
        return Err(KpError::NoBracket {
            routine: ROUTINE,
            search_limit: high,
        });
    }

    let loss_at = |diameter: f64| -> Result<ConduitHeadLoss> {
        conduit_head_loss(&ConduitInput {
            barrel: Barrel::Circular {
                diameter: Length::new(diameter)?,
            },
            discharge,
            length,
            roughness,
            coefficients,
            velocity_limits,
        })
    };

    if loss_at(high)?.total_head_loss.value() > allowance {
        return Err(KpError::NoBracket {
            routine: ROUTINE,
            search_limit: high,
        });
    }
    if loss_at(low)?.total_head_loss.value() <= allowance {
        let head_loss = loss_at(low)?;
        return Ok(BarrelSizing {
            diameter: Length::new(low)?,
            head_loss,
            allowable_head_loss,
            iterations: 0,
        });
    }

    let mut iterations = 0u32;
    while iterations < options.max_iterations {
        iterations += 1;
        let mid = 0.5 * (low + high);
        if loss_at(mid)?.total_head_loss.value() > allowance {
            low = mid;
        } else {
            high = mid;
        }
        if (high - low) <= tolerance {
            // Return the satisfying side of the bracket.
            let head_loss = loss_at(high)?;
            return Ok(BarrelSizing {
                diameter: Length::new(high)?,
                head_loss,
                allowable_head_loss,
                iterations,
            });
        }
    }

    Err(KpError::DidNotConverge {
        routine: ROUTINE,
        iterations,
        tolerance,
        residual: high - low,
    })
}
