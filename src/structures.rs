//! Discharge relations for hydraulic structures, and drop structure energy
//! dissipation.
//!
//! Reference: KP-02 (diversion headworks) and KP-04 (structures), which adopt
//! the standard weir, orifice and gate relations, and the straight drop
//! relations of Rand for a simple drop structure.
//!
//! Every discharge coefficient here is a caller input. Coefficients depend on
//! the geometry of the particular structure, on the approach conditions and on
//! the head, and the criteria give them as charts. Reading the chart is the
//! engineer's job; this module does the arithmetic that follows.
//!
//! # Units
//!
//! Heads, widths, openings and depths m, areas m², discharges m³/s, velocities
//! m/s. See [`crate::units`].
//!
//! # Disclaimer
//!
//! Every function here is a calculation aid, not a substitute for a licensed
//! engineer's judgement. A discharge relation applies only within the range of
//! head, submergence and approach conditions it was calibrated for, and the
//! result must be checked against the governing criteria.

use std::fmt;

use crate::error::{in_range, non_negative, positive, Result};
use crate::units::{guard_result, Area, Discharge, Length, Ratio, Velocity};
use crate::GRAVITY;

/// Coefficient in the Francis formula for a rectangular measuring weir,
/// `Q = 1.84 b_eff H^(3/2)` in SI units.
pub const FRANCIS_COEFFICIENT: f64 = 1.84;

/// Coefficient in the Cipoletti formula, `Q = 1.859 b H^(3/2)` in SI units.
///
/// The Cipoletti weir is a trapezoidal sharp crested weir with side slopes of
/// 1 horizontal to 4 vertical, chosen so that the side contractions make up for
/// the end contractions and the discharge stays proportional to `b H^(3/2)`.
pub const CIPOLETTI_COEFFICIENT: f64 = 1.859;

/// Discharge over a sharp crested rectangular weir, suppressed (full width).
///
/// `Q = (2/3) Cd b sqrt(2g) H^(3/2)`, with `H` the head over the crest measured
/// far enough upstream to be clear of the drawdown.
///
/// The approach velocity head is not included. Where it matters, add it to `H`
/// before calling, or use [`broad_crested_weir_discharge`] with an approach
/// velocity coefficient.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the coefficient is outside 0 to 1, or the
/// width or head is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::sharp_crested_weir_discharge;
/// use kp_irigasi::units::{Length, Ratio};
///
/// // Cd = 0.62, b = 1.50 m, H = 0.30 m
/// // Q = (2/3) × 0.62 × 1.50 × sqrt(2 × 9.81) × 0.30^1.5
/// //   = 0.62 × 4.42945 × 0.164317 = 0.451256 m³/s
/// let q = sharp_crested_weir_discharge(
///     Ratio::new(0.62)?,
///     Length::new(1.50)?,
///     Length::new(0.30)?,
/// )?;
/// assert!((q.value() - 0.451256).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn sharp_crested_weir_discharge(
    discharge_coefficient: Ratio,
    crest_width: Length,
    head: Length,
) -> Result<Discharge> {
    let cd = in_range(
        "discharge_coefficient",
        discharge_coefficient.value(),
        0.0,
        1.0,
    )?;
    let b = positive("crest_width", crest_width.value())?;
    let h = positive("head", head.value())?;
    Discharge::new(guard_result(
        "discharge",
        (2.0 / 3.0) * cd * b * (2.0 * GRAVITY).sqrt() * h.powf(1.5),
    )?)
}

/// Rehbock discharge coefficient for a sharp crested rectangular weir,
/// `Cd = 0.602 + 0.083 H/P`.
///
/// `P` is the crest height above the approach bed. The relation is an empirical
/// fit valid for a well ventilated, suppressed, sharp crested weir with
/// `H/P` up to about 1 and `H` at least about 0.03 m. Outside that range read
/// the coefficient from the criteria instead.
///
/// # Errors
///
/// Returns [`crate::KpError::NotPositive`] if the crest height is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::rehbock_coefficient;
/// use kp_irigasi::units::Length;
///
/// // H = 0.30 m, P = 0.60 m: Cd = 0.602 + 0.083 × 0.5 = 0.6435
/// let cd = rehbock_coefficient(Length::new(0.30)?, Length::new(0.60)?)?;
/// assert!((cd.value() - 0.6435).abs() < 1e-12);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn rehbock_coefficient(head: Length, crest_height: Length) -> Result<Ratio> {
    let h = non_negative("head", head.value())?;
    let p = positive("crest_height", crest_height.value())?;
    Ratio::new(guard_result("rehbock_coefficient", 0.602 + 0.083 * h / p)?)
}

/// Discharge over a broad crested weir with a rectangular control section, the
/// KP-04 form.
///
/// `Q = Cd Cv (2/3) sqrt(2g/3) b h1^(3/2)`, which evaluates to
/// `Q = 1.705 Cd Cv b h1^(3/2)` at g = 9.81 m/s². `Cd` is the discharge
/// coefficient of the crest and `Cv` the approach velocity coefficient, both
/// read from the criteria for the crest geometry and the approach section.
/// `h1` is the upstream head over the crest.
///
/// # Errors
///
/// Returns [`crate::KpError`] if a coefficient is zero or the width or head is
/// zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::broad_crested_weir_discharge;
/// use kp_irigasi::units::{Length, Ratio};
///
/// // Cd = 0.93, Cv = 1.02, b = 2.00 m, h1 = 0.45 m
/// // 1.705 × 0.93 × 1.02 × 2.00 × 0.45^1.5
/// // = 1.704895 × 0.9486 × 2.00 × 0.301869 = 0.976404 m³/s
/// let q = broad_crested_weir_discharge(
///     Ratio::new(0.93)?,
///     Ratio::new(1.02)?,
///     Length::new(2.00)?,
///     Length::new(0.45)?,
/// )?;
/// assert!((q.value() - 0.976404).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn broad_crested_weir_discharge(
    discharge_coefficient: Ratio,
    approach_velocity_coefficient: Ratio,
    crest_width: Length,
    head: Length,
) -> Result<Discharge> {
    let cd = positive("discharge_coefficient", discharge_coefficient.value())?;
    let cv = positive(
        "approach_velocity_coefficient",
        approach_velocity_coefficient.value(),
    )?;
    let b = positive("crest_width", crest_width.value())?;
    let h = positive("head", head.value())?;
    let constant = (2.0 / 3.0) * (2.0 * GRAVITY / 3.0).sqrt();
    Discharge::new(guard_result(
        "discharge",
        constant * cd * cv * b * h.powf(1.5),
    )?)
}

/// Discharge through a free orifice, `Q = Cd A sqrt(2 g H)`.
///
/// `H` is the head from the upstream water level to the centre of the orifice,
/// with free discharge downstream, that is the jet is not drowned.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the coefficient is outside 0 to 1, or the area
/// or head is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::free_orifice_discharge;
/// use kp_irigasi::units::{Area, Length, Ratio};
///
/// // Cd = 0.62, A = 0.20 m², H = 0.80 m
/// // Q = 0.62 × 0.20 × sqrt(2 × 9.81 × 0.80) = 0.124 × 3.961817 = 0.491265 m³/s
/// let q = free_orifice_discharge(
///     Ratio::new(0.62)?,
///     Area::new(0.20)?,
///     Length::new(0.80)?,
/// )?;
/// assert!((q.value() - 0.491265).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn free_orifice_discharge(
    discharge_coefficient: Ratio,
    orifice_area: Area,
    head: Length,
) -> Result<Discharge> {
    let cd = in_range(
        "discharge_coefficient",
        discharge_coefficient.value(),
        0.0,
        1.0,
    )?;
    let a = positive("orifice_area", orifice_area.value())?;
    let h = positive("head", head.value())?;
    Discharge::new(guard_result(
        "discharge",
        cd * a * (2.0 * GRAVITY * h).sqrt(),
    )?)
}

/// Discharge through a submerged orifice, `Q = Cd A sqrt(2 g dh)`.
///
/// `dh` is the difference between upstream and downstream water levels. The
/// orifice must be drowned on both sides for this to apply; if the downstream
/// level falls below the orifice soffit the flow is not fully submerged and the
/// relation no longer holds.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the coefficient is outside 0 to 1, or the area
/// or head difference is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::submerged_orifice_discharge;
/// use kp_irigasi::units::{Area, Length, Ratio};
///
/// // Cd = 0.80, A = 0.30 m², dh = 0.12 m
/// // Q = 0.80 × 0.30 × sqrt(2 × 9.81 × 0.12) = 0.24 × 1.534405 = 0.368257 m³/s
/// let q = submerged_orifice_discharge(
///     Ratio::new(0.80)?,
///     Area::new(0.30)?,
///     Length::new(0.12)?,
/// )?;
/// assert!((q.value() - 0.368257).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn submerged_orifice_discharge(
    discharge_coefficient: Ratio,
    orifice_area: Area,
    head_difference: Length,
) -> Result<Discharge> {
    let cd = in_range(
        "discharge_coefficient",
        discharge_coefficient.value(),
        0.0,
        1.0,
    )?;
    let a = positive("orifice_area", orifice_area.value())?;
    let dh = positive("head_difference", head_difference.value())?;
    Discharge::new(guard_result(
        "discharge",
        cd * a * (2.0 * GRAVITY * dh).sqrt(),
    )?)
}

/// Discharge under a sluice gate, the KP-04 form
/// `Q = K mu a b sqrt(2 g z)`.
///
/// `mu` is the discharge coefficient of the gate, `a` the gate opening, `b` the
/// gate width, `z` the driving head, and `K` the submergence factor, which is 1
/// for free flow and less than 1 when the downstream level drowns the jet. All
/// four are read from the criteria for the gate geometry and the operating
/// condition.
///
/// # Errors
///
/// Returns [`crate::KpError`] if a coefficient is outside 0 to 1, or the
/// opening, width or head is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::sluice_gate_discharge;
/// use kp_irigasi::units::{Length, Ratio};
///
/// // mu = 0.62, K = 1.0 free flow, a = 0.25 m, b = 1.20 m, z = 0.50 m
/// // Q = 1.0 × 0.62 × 0.25 × 1.20 × sqrt(2 × 9.81 × 0.50)
/// //   = 0.186 × 3.132092 = 0.582569 m³/s
/// let q = sluice_gate_discharge(
///     Ratio::new(0.62)?,
///     Ratio::new(1.0)?,
///     Length::new(0.25)?,
///     Length::new(1.20)?,
///     Length::new(0.50)?,
/// )?;
/// assert!((q.value() - 0.582569).abs() < 1e-6);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn sluice_gate_discharge(
    discharge_coefficient: Ratio,
    submergence_factor: Ratio,
    opening: Length,
    width: Length,
    head: Length,
) -> Result<Discharge> {
    let mu = in_range(
        "discharge_coefficient",
        discharge_coefficient.value(),
        0.0,
        1.0,
    )?;
    let k = in_range("submergence_factor", submergence_factor.value(), 0.0, 1.0)?;
    let a = positive("opening", opening.value())?;
    let b = positive("width", width.value())?;
    let z = positive("head", head.value())?;
    Discharge::new(guard_result(
        "discharge",
        k * mu * a * b * (2.0 * GRAVITY * z).sqrt(),
    )?)
}

/// Discharge over a Cipoletti measuring weir, `Q = 1.859 b H^(3/2)`.
///
/// `b` is the crest width at the bottom of the trapezoidal notch and `H` the
/// head over the crest. The formula assumes a fully ventilated nappe, an
/// approach channel wide and deep enough for the approach velocity to be
/// negligible, and a head within the range the weir was calibrated over. KP-04
/// gives those limits; check the installation against them.
///
/// # Errors
///
/// Returns [`crate::KpError::NotPositive`] if the width or head is zero.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::cipoletti_weir_discharge;
/// use kp_irigasi::units::Length;
///
/// // b = 0.50 m, H = 0.20 m: Q = 1.859 × 0.50 × 0.20^1.5
/// //   = 0.9295 × 0.0894427 = 0.0831370 m³/s = 83.137 l/s
/// let q = cipoletti_weir_discharge(Length::new(0.50)?, Length::new(0.20)?)?;
/// assert!((q.as_litres_per_second() - 83.1370).abs() < 1e-3);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn cipoletti_weir_discharge(crest_width: Length, head: Length) -> Result<Discharge> {
    let b = positive("crest_width", crest_width.value())?;
    let h = positive("head", head.value())?;
    Discharge::new(guard_result(
        "discharge",
        CIPOLETTI_COEFFICIENT * b * h.powf(1.5),
    )?)
}

/// Discharge over a rectangular measuring weir by the Francis formula,
/// `Q = 1.84 b_eff H^(3/2)`.
///
/// `b_eff = b - 0.1 n H`, where `n` is the number of end contractions: 2 for a
/// notch narrower than the approach channel, 0 for a suppressed weir that spans
/// it. The effective width must stay positive, so the head cannot approach the
/// crest width.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the width or head is zero, if `n` is greater
/// than 2, or if the contraction correction drives the effective width to zero
/// or below.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::rectangular_weir_discharge;
/// use kp_irigasi::units::Length;
///
/// // b = 0.60 m, H = 0.15 m, two end contractions:
/// // b_eff = 0.60 - 0.1 × 2 × 0.15 = 0.57 m
/// // Q = 1.84 × 0.57 × 0.15^1.5 = 1.0488 × 0.0580948 = 0.0609298 m³/s
/// let q = rectangular_weir_discharge(Length::new(0.60)?, Length::new(0.15)?, 2)?;
/// assert!((q.as_litres_per_second() - 60.9298).abs() < 1e-3);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn rectangular_weir_discharge(
    crest_width: Length,
    head: Length,
    end_contractions: u8,
) -> Result<Discharge> {
    let b = positive("crest_width", crest_width.value())?;
    let h = positive("head", head.value())?;
    if end_contractions > 2 {
        return Err(crate::KpError::OutOfRange {
            parameter: "end_contractions",
            value: f64::from(end_contractions),
            min: 0.0,
            max: 2.0,
        });
    }
    let effective_width = positive(
        "effective_crest_width",
        b - 0.1 * f64::from(end_contractions) * h,
    )?;
    Discharge::new(guard_result(
        "discharge",
        FRANCIS_COEFFICIENT * effective_width * h.powf(1.5),
    )?)
}

/// Discharge over a V notch (Thomson) weir,
/// `Q = (8/15) Cd sqrt(2g) tan(theta/2) H^(5/2)`.
///
/// `theta` is the full notch angle in degrees, usually 90. The V notch is the
/// accurate choice for small discharges at a tertiary offtake, where a
/// rectangular weir would need an unreadably small head.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the coefficient is outside 0 to 1, the head is
/// zero, or the angle is outside 10 to 170 degrees.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::v_notch_weir_discharge;
/// use kp_irigasi::units::{Length, Ratio};
///
/// // 90 degree notch, Cd = 0.60, H = 0.20 m
/// // (8/15) × 0.60 × sqrt(19.62) × tan(45°) = 0.32 × 4.42945 = 1.41742
/// // Q = 1.417419 × 0.20^2.5 = 1.417419 × 0.0178885 = 0.0253556 m³/s
/// let q = v_notch_weir_discharge(Ratio::new(0.60)?, 90.0, Length::new(0.20)?)?;
/// assert!((q.as_litres_per_second() - 25.3556).abs() < 1e-3);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn v_notch_weir_discharge(
    discharge_coefficient: Ratio,
    notch_angle_degrees: f64,
    head: Length,
) -> Result<Discharge> {
    let cd = in_range(
        "discharge_coefficient",
        discharge_coefficient.value(),
        0.0,
        1.0,
    )?;
    let angle = in_range("notch_angle_degrees", notch_angle_degrees, 10.0, 170.0)?;
    let h = positive("head", head.value())?;
    let half_angle = angle.to_radians() / 2.0;
    Discharge::new(guard_result(
        "discharge",
        (8.0 / 15.0) * cd * (2.0 * GRAVITY).sqrt() * half_angle.tan() * h.powf(2.5),
    )?)
}

/// A hydraulic jump on a horizontal floor, from the upstream (supercritical)
/// side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HydraulicJump {
    /// Depth before the jump, m.
    pub initial_depth: Length,
    /// Velocity before the jump, m/s.
    pub initial_velocity: Velocity,
    /// Froude number before the jump. Must exceed 1 for a jump to form.
    pub initial_froude: f64,
    /// Sequent depth after the jump, m.
    pub sequent_depth: Length,
    /// Velocity after the jump, m/s.
    pub sequent_velocity: Velocity,
    /// Specific energy before the jump, m.
    pub initial_energy: Length,
    /// Specific energy after the jump, m.
    pub sequent_energy: Length,
    /// Energy dissipated in the jump, m, from `(y2 - y1)³ / (4 y1 y2)`.
    pub energy_loss: Length,
}

/// Solves a hydraulic jump in a rectangular section from the upstream depth and
/// velocity.
///
/// The sequent depth follows from momentum,
/// `y2/y1 = 0.5 (sqrt(1 + 8 Fr1²) - 1)`, and the energy loss from
/// `dE = (y2 - y1)³ / (4 y1 y2)`. Both are exact for a rectangular horizontal
/// channel with negligible friction over the jump.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the depth or velocity is zero, or if the
/// upstream Froude number is not greater than 1, since no jump forms in
/// subcritical flow.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::hydraulic_jump;
/// use kp_irigasi::units::{Length, Velocity};
///
/// // y1 = 0.20 m, V1 = 4.00 m/s
/// // Fr1 = 4.00 / sqrt(9.81 × 0.20) = 4.00 / 1.40071 = 2.85570
/// // y2 = 0.5 × 0.20 × (sqrt(1 + 8 × 8.154944) - 1)
/// //    = 0.10 × (sqrt(66.23955) - 1) = 0.10 × 7.138768 = 0.7138768 m
/// // dE = (0.5138768)³ / (4 × 0.20 × 0.7138768)
/// //    = 0.1356945 / 0.5711015 = 0.2376095 m
/// let jump = hydraulic_jump(Length::new(0.20)?, Velocity::new(4.00)?)?;
/// assert!((jump.sequent_depth.value() - 0.7138768).abs() < 1e-7);
/// assert!((jump.energy_loss.value() - 0.2376095).abs() < 1e-7);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn hydraulic_jump(initial_depth: Length, initial_velocity: Velocity) -> Result<HydraulicJump> {
    let y1 = positive("initial_depth", initial_depth.value())?;
    let v1 = positive("initial_velocity", initial_velocity.value())?;
    let fr1 = guard_result("initial_froude", v1 / (GRAVITY * y1).sqrt())?;
    if fr1 <= 1.0 {
        return Err(crate::KpError::OutOfRange {
            parameter: "initial_froude",
            value: fr1,
            min: 1.0,
            max: f64::INFINITY,
        });
    }
    let y2 = 0.5 * y1 * ((1.0 + 8.0 * fr1 * fr1).sqrt() - 1.0);
    let v2 = guard_result("sequent_velocity", v1 * y1 / y2)?;
    let e1 = y1 + v1 * v1 / (2.0 * GRAVITY);
    let e2 = y2 + v2 * v2 / (2.0 * GRAVITY);
    let loss = (y2 - y1).powi(3) / (4.0 * y1 * y2);
    Ok(HydraulicJump {
        initial_depth,
        initial_velocity,
        initial_froude: fr1,
        sequent_depth: Length::new(y2)?,
        sequent_velocity: Velocity::new(v2)?,
        initial_energy: Length::new(e1)?,
        sequent_energy: Length::new(e2)?,
        energy_loss: Length::new(loss)?,
    })
}

/// Default ratio of jump length to the depth rise `y2 - y1` used when sizing a
/// straight drop basin.
///
/// A working value of 6 is common for a free jump on a flat apron. The value to
/// use is the engineer's decision against the criteria for the basin type
/// chosen.
pub const DEFAULT_JUMP_LENGTH_FACTOR: f64 = 6.0;

/// A straight drop structure and its stilling basin, basic case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StraightDrop {
    /// Unit discharge over the crest, m³/s per metre width.
    pub unit_discharge: f64,
    /// Height of the drop, m.
    pub drop_height: Length,
    /// Critical depth on the crest, `yc = (q²/g)^(1/3)`, m.
    pub critical_depth: Length,
    /// Drop number `D = q² / (g dz³)`, dimensionless.
    pub drop_number: f64,
    /// Horizontal distance from the drop wall to the point where the nappe
    /// strikes the floor, m.
    pub nappe_length: Length,
    /// Pool depth behind the falling nappe, m.
    pub pool_depth: Length,
    /// Depth at the toe of the nappe, before the jump, m.
    pub toe_depth: Length,
    /// Sequent depth after the jump, m.
    pub sequent_depth: Length,
    /// Estimated jump length, `factor × (y2 - y1)`, m.
    pub jump_length: Length,
    /// Total basin length, nappe length plus jump length, m.
    pub basin_length: Length,
    /// Energy dissipated in the jump, m.
    pub energy_loss: Length,
}

impl fmt::Display for StraightDrop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Straight drop structure, KP-04 basic case")?;
        writeln!(
            f,
            "  unit discharge q       {:>9.4} m³/s per m",
            self.unit_discharge
        )?;
        writeln!(
            f,
            "  drop height dz         {:>9.3} m",
            self.drop_height.value()
        )?;
        writeln!(
            f,
            "  critical depth yc      {:>9.3} m",
            self.critical_depth.value()
        )?;
        writeln!(f, "  drop number D          {:>9.5}", self.drop_number)?;
        writeln!(
            f,
            "  nappe length Ld        {:>9.3} m",
            self.nappe_length.value()
        )?;
        writeln!(
            f,
            "  pool depth yp          {:>9.3} m",
            self.pool_depth.value()
        )?;
        writeln!(
            f,
            "  toe depth y1           {:>9.3} m",
            self.toe_depth.value()
        )?;
        writeln!(
            f,
            "  sequent depth y2       {:>9.3} m",
            self.sequent_depth.value()
        )?;
        writeln!(
            f,
            "  jump length            {:>9.3} m",
            self.jump_length.value()
        )?;
        writeln!(
            f,
            "  basin length           {:>9.3} m",
            self.basin_length.value()
        )?;
        write!(
            f,
            "  energy loss            {:>9.3} m",
            self.energy_loss.value()
        )
    }
}

/// Sizes a straight drop structure by the Rand relations, the basic case KP-04
/// adopts for a simple vertical drop.
///
/// With the drop number `D = q²/(g dz³)`,
///
/// ```text
/// Ld/dz = 4.30 D^0.27      nappe length
/// yp/dz = 1.00 D^0.22      pool depth behind the nappe
/// y1/dz = 0.54 D^0.425     depth at the toe
/// y2/dz = 1.66 D^0.27      sequent depth
/// ```
///
/// These are empirical fits for a free, ventilated nappe falling onto a
/// horizontal floor. They do not cover a drowned nappe, a sloping apron, or a
/// baffled basin: for those, use the basin type the criteria specify.
///
/// The jump length is `factor × (y2 - y1)`, see
/// [`DEFAULT_JUMP_LENGTH_FACTOR`], and the basin length is the nappe length
/// plus the jump length. The energy loss is
/// `(y2 - y1)³ / (4 y1 y2)`.
///
/// # Errors
///
/// Returns [`crate::KpError::NotPositive`] if the unit discharge, drop height
/// or jump length factor is zero.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// Basin dimensions must be checked against the governing criteria, and the
/// structure needs cut off walls, ventilation of the nappe and downstream
/// protection that this calculation says nothing about.
///
/// # Examples
///
/// ```
/// use kp_irigasi::structures::{straight_drop, DEFAULT_JUMP_LENGTH_FACTOR};
/// use kp_irigasi::units::Length;
///
/// // q = 0.60 m³/s per m, dz = 1.00 m
/// // D = 0.36 / (9.81 × 1.0) = 0.0366972
/// // yc = (0.36 / 9.81)^(1/3) = 0.3323108 m
/// // D^0.27 = 0.4096859, D^0.425 = 0.2454535
/// // Ld = 4.30 × 0.4096859 = 1.761649 m
/// // y1 = 0.54 × 0.2454535 = 0.1325449 m
/// // y2 = 1.66 × 0.4096859 = 0.6800786 m
/// let drop = straight_drop(0.60, Length::new(1.00)?, DEFAULT_JUMP_LENGTH_FACTOR)?;
/// assert!((drop.critical_depth.value() - 0.3323108).abs() < 1e-7);
/// assert!((drop.nappe_length.value() - 1.761649).abs() < 1e-6);
/// assert!((drop.toe_depth.value() - 0.1325449).abs() < 1e-7);
/// assert!((drop.sequent_depth.value() - 0.6800786).abs() < 1e-7);
/// # Ok::<(), kp_irigasi::KpError>(())
/// ```
pub fn straight_drop(
    unit_discharge: f64,
    drop_height: Length,
    jump_length_factor: f64,
) -> Result<StraightDrop> {
    let q = positive("unit_discharge", unit_discharge)?;
    let dz = positive("drop_height", drop_height.value())?;
    let factor = positive("jump_length_factor", jump_length_factor)?;

    let drop_number = guard_result("drop_number", q * q / (GRAVITY * dz.powi(3)))?;
    let critical_depth = guard_result("critical_depth", (q * q / GRAVITY).cbrt())?;
    let nappe_length = 4.30 * dz * drop_number.powf(0.27);
    let pool_depth = 1.00 * dz * drop_number.powf(0.22);
    let toe_depth = positive("toe_depth", 0.54 * dz * drop_number.powf(0.425))?;
    let sequent_depth = positive("sequent_depth", 1.66 * dz * drop_number.powf(0.27))?;
    let jump_length = factor * (sequent_depth - toe_depth);
    let energy_loss = (sequent_depth - toe_depth).powi(3) / (4.0 * toe_depth * sequent_depth);

    Ok(StraightDrop {
        unit_discharge: q,
        drop_height,
        critical_depth: Length::new(critical_depth)?,
        drop_number,
        nappe_length: Length::new(nappe_length)?,
        pool_depth: Length::new(pool_depth)?,
        toe_depth: Length::new(toe_depth)?,
        sequent_depth: Length::new(sequent_depth)?,
        jump_length: Length::new(jump_length)?,
        basin_length: Length::new(nappe_length + jump_length)?,
        energy_loss: Length::new(energy_loss)?,
    })
}
