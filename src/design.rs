//! Canal section design.
//!
//! Reference: KP-03, hydraulic design of irrigation canals. Given a design
//! discharge, a bed slope, a side slope and a roughness, this module sizes a
//! trapezoidal section and returns every value an engineer needs to check the
//! result by hand.
//!
//! # Units
//!
//! Discharge m³/s, lengths m, velocity m/s, slope m/m. See [`crate::units`].
//!
//! # Disclaimer
//!
//! Every routine here is a calculation aid. It is not a substitute for a
//! licensed engineer's judgement. The section it produces must be checked
//! against the governing Kriteria Perencanaan, against the project's own
//! standards, and against site conditions the calculation knows nothing about,
//! including soil type, groundwater, construction method and maintenance
//! access.

use std::fmt;

use crate::check::{Check, CheckList, CheckStatus};
use crate::error::{non_negative, positive, Result};
use crate::freeboard::FreeboardRule;
use crate::section::{SectionGeometry, Trapezoid};
use crate::uniform_flow::{
    normal_depth, uniform_flow, FlowRegime, NormalDepthOptions, UniformFlow, VelocityLimits,
};
use crate::units::{guard_result, Discharge, Length, ManningN, SideSlope, Slope};

/// Relative tolerance on reproducing the design discharge, used for the
/// discharge reproduction check in [`design_channel`].
///
/// The sized section is put back through Manning's equation and the discharge
/// it carries is compared with the design discharge. Agreement to one part in
/// 10 000 confirms the sizing arithmetic; a failure here means a bug or a
/// pathological input, not a design problem.
pub const DISCHARGE_TOLERANCE: f64 = 1.0e-4;

/// How the bed width and the flow depth are to be proportioned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SectionProportion {
    /// The most hydraulically efficient trapezoid for the given side slope,
    /// that is the one with the smallest wetted perimeter for the area, giving
    /// `b/h = 2(sqrt(1+m²) - m)` and `R = h/2`.
    ///
    /// The efficient section is deep and narrow. KP-03 canals are usually
    /// wider than the efficient shape, for stability, freeboard economy and
    /// maintenance access, so treat this as a starting point rather than an
    /// answer.
    MostEfficient,
    /// A fixed bed width to depth ratio `b/h`, which is how KP-03 canal
    /// sections are normally proportioned. The ratio against discharge is a
    /// KP-03 lookup and is supplied by the caller.
    BedWidthDepthRatio(f64),
    /// A bed width fixed by the caller, with the depth solved from the
    /// discharge.
    FixedBedWidth(Length),
}

/// The efficient trapezoid geometry for a given side slope, independent of
/// discharge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfficientTrapezoid {
    /// Side slope m, metres horizontal per metre vertical.
    pub side_slope: SideSlope,
    /// The efficient bed width to depth ratio `b/h = 2(sqrt(1+m²) - m)`.
    pub bed_width_depth_ratio: f64,
    /// Area coefficient `ca` in `A = ca h²`, equal to `2 sqrt(1+m²) - m`.
    pub area_coefficient: f64,
}

/// Returns the proportions of the most hydraulically efficient trapezoid for a
/// side slope.
///
/// For a fixed area, the trapezoid with the least wetted perimeter satisfies
/// `b = 2h(sqrt(1+m²) - m)`, and then `R = h/2` exactly. Setting m = 0 recovers
/// the efficient rectangle, `b = 2h`.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the side slope is not finite.
///
/// # Examples
///
/// ```
/// use kp_irigasi::design::efficient_trapezoid;
/// use kp_irigasi::units::SideSlope;
///
/// // m = 1.5: sqrt(1 + 2.25) = 1.802776
/// // b/h = 2 × (1.802776 - 1.5) = 0.605551
/// // A/h² = 2 × 1.802776 - 1.5 = 2.105551
/// let e = efficient_trapezoid(SideSlope::new(1.5)?)?;
/// assert!((e.bed_width_depth_ratio - 0.605551).abs() < 1e-6);
/// assert!((e.area_coefficient - 2.105551).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn efficient_trapezoid(side_slope: SideSlope) -> Result<EfficientTrapezoid> {
    let m = non_negative("side_slope", side_slope.value())?;
    let root = (1.0 + m * m).sqrt();
    Ok(EfficientTrapezoid {
        side_slope,
        bed_width_depth_ratio: guard_result("bed_width_depth_ratio", 2.0 * (root - m))?,
        area_coefficient: guard_result("area_coefficient", 2.0 * root - m)?,
    })
}

/// Everything the caller supplies to [`design_channel`].
///
/// The freeboard rule and the velocity limits are references to caller-supplied
/// data, because both are KP lookups that belong to the project rather than to
/// this crate.
pub struct ChannelDesignInput<'a> {
    /// Design discharge, m³/s.
    pub design_discharge: Discharge,
    /// Bed slope, m/m. Must be greater than zero.
    pub bed_slope: Slope,
    /// Side slope m, metres horizontal per metre vertical, from the KP-03
    /// minimum for the soil type.
    pub side_slope: SideSlope,
    /// Manning roughness n, from the KP-03 value for the lining and canal size.
    pub roughness: ManningN,
    /// How to proportion the section.
    pub proportion: SectionProportion,
    /// Permissible velocity range for the bed or lining material.
    pub velocity_limits: VelocityLimits,
    /// The freeboard rule to apply.
    pub freeboard_rule: &'a dyn FreeboardRule,
    /// Optional maximum Froude number. KP-03 keeps canal flow well clear of
    /// critical; a limit around 0.5 to 0.7 is common practice, but the value is
    /// the engineer's decision and is not embedded here. `None` skips the check.
    pub max_froude: Option<f64>,
    /// Optional minimum freeboard, m, if the project imposes one over and above
    /// the freeboard rule. `None` skips the check.
    pub min_freeboard: Option<Length>,
    /// Solver settings for the cases that need a normal depth solve.
    pub solver: NormalDepthOptions,
}

/// The sized section together with every intermediate value and every check.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelDesign {
    /// Design discharge the section was sized for, m³/s.
    pub design_discharge: Discharge,
    /// The sized trapezoidal section.
    pub section: Trapezoid,
    /// Flow depth at the design discharge, m.
    pub flow_depth: Length,
    /// Bed width, m.
    pub bed_width: Length,
    /// Bed width to depth ratio actually achieved.
    pub bed_width_depth_ratio: f64,
    /// The uniform flow state at the design discharge, carrying the geometry,
    /// the velocity, the conveyance, the Froude number and the regime.
    pub flow: UniformFlow,
    /// Freeboard from the freeboard rule, m.
    pub freeboard: Length,
    /// Total canal depth, flow depth plus freeboard, m.
    pub total_depth: Length,
    /// Water surface width at the design discharge, m.
    pub top_width: Length,
    /// Section width at bank level, `b + 2 m (h + F)`, m. This is the width to
    /// set out, not the water surface width.
    pub top_width_at_bank: Length,
    /// Discharge the sized section actually carries by Manning, m³/s. Compare
    /// with the design discharge as an independent check.
    pub computed_discharge: Discharge,
    /// Number of solver iterations, zero where the section was sized in closed
    /// form.
    pub solver_iterations: u32,
    /// Every design check, passed or failed.
    pub checks: Vec<Check>,
}

impl ChannelDesign {
    /// The section geometry at the design flow depth.
    pub fn geometry(&self) -> SectionGeometry {
        self.flow.geometry
    }

    /// Returns true when no check failed.
    pub fn all_checks_pass(&self) -> bool {
        self.checks.all_pass()
    }

    /// Returns the failed checks.
    pub fn failures(&self) -> Vec<&Check> {
        self.checks.failures()
    }
}

impl fmt::Display for ChannelDesign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let g = self.flow.geometry;
        writeln!(f, "Trapezoidal canal section, KP-03 uniform flow design")?;
        writeln!(
            f,
            "  design discharge Q       {:>10.4} m³/s",
            self.design_discharge.value()
        )?;
        writeln!(
            f,
            "  bed slope S              {:>10.6} m/m",
            self.flow.slope.value()
        )?;
        writeln!(
            f,
            "  side slope m             {:>10.3} h:1v",
            self.section.side_slope.value()
        )?;
        writeln!(
            f,
            "  roughness n              {:>10.4} s/m^(1/3)",
            self.flow.roughness.value()
        )?;
        writeln!(
            f,
            "  bed width b              {:>10.3} m",
            self.bed_width.value()
        )?;
        writeln!(
            f,
            "  flow depth h             {:>10.3} m",
            self.flow_depth.value()
        )?;
        writeln!(
            f,
            "  b/h ratio                {:>10.3}",
            self.bed_width_depth_ratio
        )?;
        writeln!(f, "  flow area A              {:>10.3} m²", g.area.value())?;
        writeln!(
            f,
            "  wetted perimeter P       {:>10.3} m",
            g.wetted_perimeter.value()
        )?;
        writeln!(
            f,
            "  hydraulic radius R       {:>10.3} m",
            g.hydraulic_radius.value()
        )?;
        writeln!(
            f,
            "  top width T              {:>10.3} m",
            self.top_width.value()
        )?;
        writeln!(
            f,
            "  hydraulic depth D = A/T  {:>10.3} m",
            g.hydraulic_depth.value()
        )?;
        writeln!(
            f,
            "  mean velocity V          {:>10.3} m/s",
            self.flow.velocity.value()
        )?;
        writeln!(
            f,
            "  Froude number Fr         {:>10.3}  {}",
            self.flow.froude_number, self.flow.regime
        )?;
        writeln!(
            f,
            "  freeboard F              {:>10.3} m",
            self.freeboard.value()
        )?;
        writeln!(
            f,
            "  total depth h + F        {:>10.3} m",
            self.total_depth.value()
        )?;
        writeln!(
            f,
            "  width at bank level      {:>10.3} m",
            self.top_width_at_bank.value()
        )?;
        writeln!(
            f,
            "  computed discharge       {:>10.4} m³/s",
            self.computed_discharge.value()
        )?;
        writeln!(
            f,
            "  solver iterations        {:>10}",
            self.solver_iterations
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

/// Sizes a trapezoidal canal section for a design discharge and returns every
/// check.
///
/// The depth is found in closed form for [`SectionProportion::MostEfficient`]
/// and [`SectionProportion::BedWidthDepthRatio`], and by the bisection solver
/// of [`normal_depth`] for [`SectionProportion::FixedBedWidth`].
///
/// A failed check does not stop the calculation. The section is returned with
/// the failure recorded, because an engineer needs to see by how much a limit
/// is exceeded in order to decide what to change. Always inspect
/// [`ChannelDesign::all_checks_pass`] before using the result.
///
/// # Errors
///
/// - [`crate::KpError::NotPositive`] if the discharge, slope or roughness is
///   zero.
/// - [`crate::KpError::NoBracket`] or [`crate::KpError::DidNotConverge`] from
///   the normal depth solve for a fixed bed width.
/// - Any error from the freeboard rule.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// The section must be checked against the governing criteria before use.
///
/// # Examples
///
/// ```
/// use kp_irigasi::design::{design_channel, ChannelDesignInput, SectionProportion};
/// use kp_irigasi::freeboard::FreeboardTable;
/// use kp_irigasi::uniform_flow::{NormalDepthOptions, VelocityLimits};
/// use kp_irigasi::units::{Discharge, ManningN, SideSlope, Slope};
///
/// let freeboard = FreeboardTable::default();
/// let design = design_channel(&ChannelDesignInput {
///     design_discharge: Discharge::new(1.5)?,
///     bed_slope: Slope::new(0.0005)?,
///     side_slope: SideSlope::new(1.5)?,
///     roughness: ManningN::new(0.025)?,
///     proportion: SectionProportion::BedWidthDepthRatio(2.0),
///     velocity_limits: VelocityLimits::from_range(0.25, 0.70)?,
///     freeboard_rule: &freeboard,
///     max_froude: Some(0.7),
///     min_freeboard: None,
///     solver: NormalDepthOptions::default(),
/// })?;
///
/// assert!(design.all_checks_pass());
/// assert!((design.computed_discharge.value() - 1.5).abs() < 1e-4);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn design_channel(input: &ChannelDesignInput<'_>) -> Result<ChannelDesign> {
    let q = positive("design_discharge", input.design_discharge.value())?;
    let s = positive("bed_slope", input.bed_slope.value())?;
    let n = positive("roughness", input.roughness.value())?;
    let m = non_negative("side_slope", input.side_slope.value())?;

    // Required conveyance times n: A R^(2/3) = Q n / sqrt(S).
    let shape_factor = guard_result("shape_factor", q * n / s.sqrt())?;

    let (section, depth, iterations) = match input.proportion {
        SectionProportion::MostEfficient => {
            let efficient = efficient_trapezoid(input.side_slope)?;
            // A = ca h², R = h/2, so A R^(2/3) = ca h^(8/3) / 2^(2/3).
            let c = efficient.area_coefficient / 2f64.powf(2.0 / 3.0);
            let h = guard_result("flow_depth", (shape_factor / c).powf(3.0 / 8.0))?;
            let b = efficient.bed_width_depth_ratio * h;
            (
                Trapezoid::new(Length::new(b)?, input.side_slope),
                Length::new(h)?,
                0,
            )
        }
        SectionProportion::BedWidthDepthRatio(ratio) => {
            let k = non_negative("bed_width_depth_ratio", ratio)?;
            let root = (1.0 + m * m).sqrt();
            let perimeter_coefficient = k + 2.0 * root;
            if perimeter_coefficient <= 0.0 {
                return Err(crate::KpError::NotPositive {
                    parameter: "bed_width_depth_ratio",
                    value: ratio,
                });
            }
            // A = (k + m) h², R = (k + m) h / (k + 2 sqrt(1+m²)), so
            // A R^(2/3) = (k + m)^(5/3) / (k + 2 sqrt(1+m²))^(2/3) × h^(8/3).
            let area_coefficient = positive("bed_width_depth_ratio + side_slope", k + m)?;
            let c = area_coefficient.powf(5.0 / 3.0) / perimeter_coefficient.powf(2.0 / 3.0);
            let h = guard_result("flow_depth", (shape_factor / c).powf(3.0 / 8.0))?;
            let b = k * h;
            (
                Trapezoid::new(Length::new(b)?, input.side_slope),
                Length::new(h)?,
                0,
            )
        }
        SectionProportion::FixedBedWidth(bed_width) => {
            let section = Trapezoid::new(bed_width, input.side_slope);
            let solution = normal_depth(
                &section,
                input.design_discharge,
                input.roughness,
                input.bed_slope,
                input.solver,
            )?;
            (section, solution.depth, solution.iterations)
        }
    };

    let flow = uniform_flow(&section, depth, input.roughness, input.bed_slope)?;
    let freeboard = input.freeboard_rule.freeboard(input.design_discharge)?;
    let total_depth = Length::new(depth.value() + freeboard.value())?;
    let top_width_at_bank = Length::new(section.bed_width.value() + 2.0 * m * total_depth.value())?;
    let bed_width_depth_ratio = guard_result(
        "bed_width_depth_ratio",
        section.bed_width.value() / depth.value(),
    )?;

    let mut checks = input.velocity_limits.check(flow.velocity);
    checks.push(Check::at_most(
        "Froude number",
        flow.froude_number,
        "",
        input.max_froude,
        "KP-03 canal flow to stay well below critical, limit supplied by the caller",
    ));
    checks.push(Check::verdict(
        "flow regime",
        flow.froude_number,
        "",
        if flow.regime == FlowRegime::Subcritical {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        format!(
            "uniform flow is {}, a conveyance canal must be subcritical",
            flow.regime
        ),
    ));
    checks.push(Check::at_least(
        "freeboard",
        freeboard.value(),
        "m",
        input.min_freeboard.map(Length::value),
        "project minimum freeboard over and above the freeboard rule",
    ));
    let discharge_error = (flow.discharge.value() - q).abs() / q;
    checks.push(Check::verdict(
        "discharge reproduction",
        discharge_error,
        "",
        if discharge_error <= DISCHARGE_TOLERANCE {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        format!(
            "sized section put back through Manning must return the design discharge \
             to within {DISCHARGE_TOLERANCE:e} relative"
        ),
    ));

    Ok(ChannelDesign {
        design_discharge: input.design_discharge,
        section,
        flow_depth: depth,
        bed_width: section.bed_width,
        bed_width_depth_ratio,
        freeboard,
        total_depth,
        top_width: flow.geometry.top_width,
        top_width_at_bank,
        computed_discharge: flow.discharge,
        solver_iterations: iterations,
        checks,
        flow,
    })
}
