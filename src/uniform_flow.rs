//! Uniform flow in an open channel: Manning's equation, normal depth, Froude
//! number and flow regime.
//!
//! Reference: KP-03, hydraulic design of irrigation canals, which adopts the
//! Strickler form of Manning's equation for uniform flow. The roughness
//! coefficient against lining type and canal size is a KP-03 lookup and is
//! supplied by the caller.
//!
//! # Units
//!
//! Discharge m³/s, depth and hydraulic radius m, area m², velocity m/s, slope
//! m/m, Manning n in s·m^(-1/3). See [`crate::units`].
//!
//! # Equations
//!
//! ```text
//! V = (1/n) R^(2/3) S^(1/2)          Manning
//! Q = A V = K S^(1/2),  K = (1/n) A R^(2/3)   conveyance
//! Fr = V / sqrt(g D),   D = A / T             Froude number
//! ```
//!
//! # Disclaimer
//!
//! Every function here is a calculation aid. It is not a substitute for a
//! licensed engineer's judgement, and the result must be checked against the
//! governing Kriteria Perencanaan before it is used in a design.

use crate::check::Check;
use crate::error::{positive, KpError, Result};
use crate::section::{Section, SectionGeometry};
use crate::units::{guard_result, Discharge, Length, ManningN, Slope, Velocity};
use crate::GRAVITY;

/// Default half width of the band around Fr = 1 within which flow is reported
/// as critical rather than sub or supercritical.
///
/// A band is needed because floating point equality with 1.0 is meaningless.
/// The value is a numerical convenience, not a criterion. KP-03 separately
/// advises keeping canal flow well away from critical, which is a design check,
/// see [`FlowRegime::is_near_critical`].
pub const DEFAULT_CRITICAL_BAND: f64 = 1.0e-3;

/// Flow regime from the Froude number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowRegime {
    /// Fr < 1. Tranquil flow, the normal state for an irrigation canal.
    Subcritical,
    /// Fr within the critical band of 1. Unstable, to be avoided in a canal.
    Critical,
    /// Fr > 1. Shooting flow, expected in chutes and downstream of drops, not
    /// in a conveyance canal.
    Supercritical,
}

impl FlowRegime {
    /// Classifies a Froude number using [`DEFAULT_CRITICAL_BAND`].
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the Froude number is not finite or is negative.
    pub fn classify(froude_number: f64) -> Result<Self> {
        Self::classify_with_band(froude_number, DEFAULT_CRITICAL_BAND)
    }

    /// Classifies a Froude number using a caller-supplied band around 1.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the Froude number is not finite or is negative,
    /// or the band is negative.
    pub fn classify_with_band(froude_number: f64, band: f64) -> Result<Self> {
        let fr = crate::error::non_negative("froude_number", froude_number)?;
        let band = crate::error::non_negative("band", band)?;
        Ok(if fr < 1.0 - band {
            FlowRegime::Subcritical
        } else if fr > 1.0 + band {
            FlowRegime::Supercritical
        } else {
            FlowRegime::Critical
        })
    }

    /// Returns true when the Froude number is close enough to 1 that the water
    /// surface will be unstable, using a caller-supplied band.
    ///
    /// KP-03 advises against designing a canal to run near critical. A band of
    /// 0.1, that is 0.9 to 1.1, is a common working rule, but the value to use
    /// is the engineer's decision and is not embedded here.
    pub fn is_near_critical(froude_number: f64, band: f64) -> bool {
        (froude_number - 1.0).abs() <= band
    }
}

impl std::fmt::Display for FlowRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            FlowRegime::Subcritical => "subcritical",
            FlowRegime::Critical => "critical",
            FlowRegime::Supercritical => "supercritical",
        };
        f.write_str(text)
    }
}

/// A complete uniform flow state at one depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UniformFlow {
    /// The section geometry at this depth.
    pub geometry: SectionGeometry,
    /// Bed slope, taken equal to the energy slope in uniform flow, m/m.
    pub slope: Slope,
    /// Manning roughness used, s·m^(-1/3).
    pub roughness: ManningN,
    /// Conveyance K = (1/n) A R^(2/3), m³/s per unit sqrt(slope).
    pub conveyance: f64,
    /// Mean velocity, m/s.
    pub velocity: Velocity,
    /// Discharge, m³/s.
    pub discharge: Discharge,
    /// Froude number, dimensionless.
    pub froude_number: f64,
    /// Flow regime from the Froude number.
    pub regime: FlowRegime,
}

/// Computes the uniform flow state at a given depth by Manning's equation.
///
/// This is the forward direction: geometry and slope in, discharge out.
///
/// # Errors
///
/// Returns [`KpError::NotPositive`] if the depth or the slope is zero, since
/// Manning's equation has no solution at zero energy slope, and if the geometry
/// is degenerate.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// Check the result against the governing criteria.
///
/// # Examples
///
/// ```
/// use kp_irigasi::section::Trapezoid;
/// use kp_irigasi::uniform_flow::uniform_flow;
/// use kp_irigasi::units::{Length, ManningN, SideSlope, Slope};
///
/// // b = 2.0 m, m = 1.5, h = 1.0 m, n = 0.025, S = 0.0004
/// // A = (2.0 + 1.5) × 1.0 = 3.5 m²
/// // P = 2.0 + 2 × 1.0 × sqrt(1 + 2.25) = 2.0 + 3.605551 = 5.605551 m
/// // R = 3.5 / 5.605551 = 0.624381 m
/// // V = (1/0.025) × 0.624381^(2/3) × 0.02 = 40 × 0.730522 × 0.02 = 0.584417 m/s
/// // Q = 3.5 × 0.584417 = 2.045461 m³/s
/// let section = Trapezoid::new(Length::new(2.0)?, SideSlope::new(1.5)?);
/// let flow = uniform_flow(
///     &section,
///     Length::new(1.0)?,
///     ManningN::new(0.025)?,
///     Slope::new(0.0004)?,
/// )?;
/// assert!((flow.discharge.value() - 2.045461).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn uniform_flow(
    section: &dyn Section,
    depth: Length,
    roughness: ManningN,
    slope: Slope,
) -> Result<UniformFlow> {
    let s = positive("slope", slope.value())?;
    let n = positive("roughness", roughness.value())?;
    let geometry = section.geometry(depth)?;

    let area = positive("flow_area", geometry.area.value())?;
    let radius = positive("hydraulic_radius", geometry.hydraulic_radius.value())?;

    let conveyance = guard_result("conveyance", area * radius.powf(2.0 / 3.0) / n)?;
    let discharge = Discharge::new(guard_result("discharge", conveyance * s.sqrt())?)?;
    let velocity = Velocity::new(guard_result("velocity", discharge.value() / area)?)?;

    let hydraulic_depth = positive("hydraulic_depth", geometry.hydraulic_depth.value())?;
    let froude_number = guard_result(
        "froude_number",
        velocity.value() / (GRAVITY * hydraulic_depth).sqrt(),
    )?;
    let regime = FlowRegime::classify(froude_number)?;

    Ok(UniformFlow {
        geometry,
        slope,
        roughness,
        conveyance,
        velocity,
        discharge,
        froude_number,
        regime,
    })
}

/// Froude number for a known velocity and hydraulic depth.
///
/// `Fr = V / sqrt(g D)`, where D = A/T is the hydraulic depth, not the flow
/// depth. For a rectangular section the two are equal; for any other shape they
/// are not, and using the flow depth is a common error.
///
/// # Errors
///
/// Returns [`KpError::NotPositive`] if the hydraulic depth is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::uniform_flow::froude_number;
/// use kp_irigasi::units::{Length, Velocity};
///
/// // V = 0.6 m/s, D = 0.8 m: Fr = 0.6 / sqrt(9.81 × 0.8) = 0.6 / 2.801428 = 0.2141765
/// let fr = froude_number(Velocity::new(0.6)?, Length::new(0.8)?)?;
/// assert!((fr - 0.2141765).abs() < 1e-7);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn froude_number(velocity: Velocity, hydraulic_depth: Length) -> Result<f64> {
    let d = positive("hydraulic_depth", hydraulic_depth.value())?;
    guard_result("froude_number", velocity.value() / (GRAVITY * d).sqrt())
}

/// Settings for the normal depth solver.
///
/// The solver is a bisection on flow depth. Bisection is used rather than
/// Newton because the conveyance `K(h) = (1/n) A(h) R(h)^(2/3)` is strictly
/// increasing in depth for every section in this crate, so a bracket is always
/// safe to find and bisection cannot diverge. The price is linear convergence,
/// which is irrelevant at this problem size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalDepthOptions {
    /// Convergence tolerance on the depth, m. The solve stops when the bracket
    /// is narrower than this. Default 1e-6 m, that is one thousandth of a
    /// millimetre, far finer than any construction tolerance.
    pub tolerance: f64,
    /// Maximum number of bisection steps. Default 100, which resolves a 100 m
    /// bracket to well under 1e-6 m.
    pub max_iterations: u32,
    /// Lower end of the search bracket, m. Default 1e-6 m. Must be positive,
    /// because the geometry is not defined at zero depth.
    pub min_depth: f64,
    /// Upper limit of the search, m. If the section cannot carry the discharge
    /// at this depth the solver reports [`KpError::NoBracket`] rather than
    /// searching forever. Default 100 m.
    pub max_depth: f64,
}

impl Default for NormalDepthOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-6,
            max_iterations: 100,
            min_depth: 1.0e-6,
            max_depth: 100.0,
        }
    }
}

/// Result of a normal depth solve, including how the solver got there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalDepthSolution {
    /// The normal depth, m.
    pub depth: Length,
    /// The uniform flow state at that depth. Its discharge should reproduce the
    /// target discharge to within the solver tolerance, which is a useful
    /// independent check on the answer.
    pub flow: UniformFlow,
    /// Number of bisection steps performed.
    pub iterations: u32,
    /// Half width of the final bracket, m.
    pub bracket_half_width: f64,
    /// Discharge residual at the answer, m³/s, that is computed minus target.
    pub discharge_residual: f64,
}

/// Solves for the normal depth that carries a target discharge.
///
/// This is the inverse direction: discharge and slope in, depth out. The method
/// is bisection on depth, see [`NormalDepthOptions`] for the tolerance, the
/// iteration limit and the search range.
///
/// The bracket is found by starting at `min_depth` and doubling an upper trial
/// depth until the section carries at least the target discharge. If that does
/// not happen below `max_depth` the function returns [`KpError::NoBracket`],
/// meaning the section is too small or the slope too flat for the discharge.
///
/// # Errors
///
/// - [`KpError::NotPositive`] if the slope, roughness or discharge is zero.
/// - [`KpError::NoBracket`] if no depth below `max_depth` carries the discharge.
/// - [`KpError::DidNotConverge`] if the iteration limit is reached before the
///   bracket is narrower than the tolerance. The function never returns an
///   unconverged answer.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// Check the result against the governing criteria.
///
/// # Examples
///
/// ```
/// use kp_irigasi::section::Trapezoid;
/// use kp_irigasi::uniform_flow::{normal_depth, uniform_flow, NormalDepthOptions};
/// use kp_irigasi::units::{Discharge, Length, ManningN, SideSlope, Slope};
///
/// // Round trip: the depth that carries the discharge computed at h = 1.0 m
/// // must come back as 1.0 m.
/// let section = Trapezoid::new(Length::new(2.0)?, SideSlope::new(1.5)?);
/// let n = ManningN::new(0.025)?;
/// let s = Slope::new(0.0004)?;
/// let q = uniform_flow(&section, Length::new(1.0)?, n, s)?.discharge;
///
/// let solved = normal_depth(&section, q, n, s, NormalDepthOptions::default())?;
/// assert!((solved.depth.value() - 1.0).abs() < 1e-5);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn normal_depth(
    section: &dyn Section,
    target: Discharge,
    roughness: ManningN,
    slope: Slope,
    options: NormalDepthOptions,
) -> Result<NormalDepthSolution> {
    const ROUTINE: &str = "normal_depth";

    let target_q = positive("target_discharge", target.value())?;
    positive("slope", slope.value())?;
    positive("roughness", roughness.value())?;
    let tolerance = positive("tolerance", options.tolerance)?;
    let mut low = positive("min_depth", options.min_depth)?;
    let max_depth = positive("max_depth", options.max_depth)?;
    if max_depth <= low {
        return Err(KpError::NoBracket {
            routine: ROUTINE,
            search_limit: max_depth,
        });
    }
    if options.max_iterations == 0 {
        return Err(KpError::DidNotConverge {
            routine: ROUTINE,
            iterations: 0,
            tolerance,
            residual: max_depth - low,
        });
    }

    let discharge_at = |h: f64| -> Result<f64> {
        Ok(uniform_flow(section, Length::new(h)?, roughness, slope)?
            .discharge
            .value())
    };

    // The section must not already carry the target at the lower bracket, else
    // the answer lies below the search range and the caller has asked for a
    // depth finer than min_depth resolves.
    if discharge_at(low)? >= target_q {
        return finish(section, roughness, slope, low, target_q, 0, 0.0);
    }

    // Bracket by doubling.
    let mut high = (low * 2.0).max(0.1);
    loop {
        if high > max_depth {
            return Err(KpError::NoBracket {
                routine: ROUTINE,
                search_limit: max_depth,
            });
        }
        if discharge_at(high)? >= target_q {
            break;
        }
        high *= 2.0;
    }

    // Bisect.
    let mut iterations = 0u32;
    while iterations < options.max_iterations {
        iterations += 1;
        let mid = 0.5 * (low + high);
        if discharge_at(mid)? < target_q {
            low = mid;
        } else {
            high = mid;
        }
        if (high - low) <= tolerance {
            let depth = 0.5 * (low + high);
            return finish(
                section,
                roughness,
                slope,
                depth,
                target_q,
                iterations,
                0.5 * (high - low),
            );
        }
    }

    Err(KpError::DidNotConverge {
        routine: ROUTINE,
        iterations,
        tolerance,
        residual: high - low,
    })
}

#[allow(clippy::too_many_arguments)]
fn finish(
    section: &dyn Section,
    roughness: ManningN,
    slope: Slope,
    depth: f64,
    target_q: f64,
    iterations: u32,
    bracket_half_width: f64,
) -> Result<NormalDepthSolution> {
    let depth = Length::new(depth)?;
    let flow = uniform_flow(section, depth, roughness, slope)?;
    Ok(NormalDepthSolution {
        depth,
        discharge_residual: flow.discharge.value() - target_q,
        flow,
        iterations,
        bracket_half_width,
    })
}

/// Permissible velocity limits for a canal, both supplied by the caller.
///
/// KP-03 gives maximum permissible velocity against bed and lining material,
/// and a minimum velocity to keep silt moving. Both are read off the criteria
/// for the material at hand; neither table is embedded in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VelocityLimits {
    /// Minimum velocity to avoid sedimentation, m/s. `None` skips the check.
    pub minimum: Option<Velocity>,
    /// Maximum velocity to avoid scour of the bed or lining, m/s. `None` skips
    /// the check.
    pub maximum: Option<Velocity>,
}

impl VelocityLimits {
    /// Creates a pair of limits.
    pub const fn new(minimum: Option<Velocity>, maximum: Option<Velocity>) -> Self {
        Self { minimum, maximum }
    }

    /// Creates limits from plain m/s figures.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if either value is not finite or is negative.
    pub fn from_range(minimum: f64, maximum: f64) -> Result<Self> {
        Ok(Self {
            minimum: Some(Velocity::new(minimum)?),
            maximum: Some(Velocity::new(maximum)?),
        })
    }

    /// Produces the two velocity checks for a computed mean velocity.
    ///
    /// The minimum check guards against sedimentation, the maximum against
    /// scour. A limit left as `None` is recorded as not checked rather than
    /// silently omitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use kp_irigasi::check::{CheckList, CheckStatus};
    /// use kp_irigasi::uniform_flow::VelocityLimits;
    /// use kp_irigasi::units::Velocity;
    ///
    /// let limits = VelocityLimits::from_range(0.30, 0.70)?;
    /// let checks = limits.check(Velocity::new(0.58)?);
    /// assert!(checks.all_pass());
    ///
    /// let checks = limits.check(Velocity::new(0.95)?);
    /// assert_eq!(checks[1].status, CheckStatus::Fail);
    /// # Ok::<(), kp_irigasi::KpError>(())
    /// ```
    pub fn check(&self, velocity: Velocity) -> Vec<Check> {
        vec![
            Check::at_least(
                "minimum velocity",
                velocity.value(),
                "m/s",
                self.minimum.map(Velocity::value),
                "minimum velocity against sedimentation, KP-03 for canals and KP-04 for \
                 closed conduits, limit supplied by the caller",
            ),
            Check::at_most(
                "maximum velocity",
                velocity.value(),
                "m/s",
                self.maximum.map(Velocity::value),
                "maximum permissible velocity for the bed, lining or barrel material, \
                 KP-03 for canals and KP-04 for closed conduits, limit supplied by the caller",
            ),
        ]
    }
}
