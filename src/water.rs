//! Irrigation water requirement, from the field up to the intake.
//!
//! Reference: KP-01, kebutuhan air (water requirement). The chain is
//!
//! ```text
//! NFR = Etc + P + WLR - Re                      net field requirement, mm/day
//! DR  = NFR / (e_tertiary × e_secondary × e_primary)   diversion requirement
//! Q   = DR × A / 8.64                            intake discharge, l/s
//! ```
//!
//! where `Etc = kc × Eto` is the crop water requirement, `P` percolation, `WLR`
//! the water layer replacement spread over its period, and `Re` effective
//! rainfall.
//!
//! # Units
//!
//! Depth rates are mm/day and are typed as [`DepthRate`], standing water layers
//! are mm and are typed as [`WaterLayer`], areas are hectares and are typed as
//! [`Hectares`], discharges are m³/s and are typed as [`Discharge`]. The
//! conversion 1 mm/day over 1 ha = 1/8.64 l/s is done only inside
//! [`DepthRate::over_area`], never by hand.
//!
//! # Disclaimer
//!
//! Every routine here is a calculation aid, not a substitute for a licensed
//! engineer's judgement. Crop coefficients, percolation rates, cropping
//! patterns and efficiencies are project data, and the resulting requirement
//! must be checked against the governing criteria and against the water
//! balance of the scheme.

use std::fmt;

use crate::error::{in_range, non_negative, positive, Result};
use crate::units::{
    checked_balance, guard_result, Days, DepthRate, Discharge, Hectares, Ratio, WaterLayer,
};

/// Default ratio of open water evaporation during land preparation to reference
/// evapotranspiration, `Eo = 1.1 Eto`.
///
/// A widely published working value used with the van de Goor and Zijlstra
/// method. Confirm it against the current KP-01 edition.
pub const DEFAULT_EO_FACTOR: f64 = 1.1;

/// Default coefficient in the effective rainfall for rice, `Re = 0.7 R80 / d`.
///
/// A widely published working value. Confirm it against the current KP-01
/// edition, and note that the coefficient for palawija (non rice secondary
/// crops) is different and is read from a table, so it is a caller input here.
pub const DEFAULT_RICE_RAINFALL_COEFFICIENT: f64 = 0.7;

/// Crop water requirement `Etc = kc × Eto`.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the crop coefficient is negative or the result
/// is not finite.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::units::{DepthRate, Ratio};
/// use imaji_kp_hydraulics::water::crop_water_requirement;
///
/// // kc = 1.10, Eto = 4.60 mm/day: Etc = 5.06 mm/day
/// let etc = crop_water_requirement(Ratio::new(1.10)?, DepthRate::new(4.60)?)?;
/// assert!((etc.value() - 5.06).abs() < 1e-12);
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn crop_water_requirement(
    crop_coefficient: Ratio,
    reference_et: DepthRate,
) -> Result<DepthRate> {
    let kc = non_negative("crop_coefficient", crop_coefficient.value())?;
    DepthRate::new(guard_result(
        "crop_water_requirement",
        kc * reference_et.value(),
    )?)
}

/// Effective rainfall for rice, `Re = coefficient × R80 / period`.
///
/// `R80` is the rainfall with an 80 percent exceedance probability over the
/// period, in mm. KP-01 uses a fixed fraction of it for rice; pass
/// [`DEFAULT_RICE_RAINFALL_COEFFICIENT`] unless the project says otherwise.
/// For palawija the fraction is read from a table against the monthly rainfall
/// and the crop, so supply that fraction instead.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the coefficient is outside 0 to 1, or the
/// period is zero.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::units::{Days, WaterLayer};
/// use imaji_kp_hydraulics::water::{effective_rainfall, DEFAULT_RICE_RAINFALL_COEFFICIENT};
///
/// // R80 = 90 mm over a 15 day half month, coefficient 0.7:
/// // Re = 0.7 × 90 / 15 = 4.2 mm/day
/// let re = effective_rainfall(
///     WaterLayer::new(90.0)?,
///     Days::new(15.0)?,
///     DEFAULT_RICE_RAINFALL_COEFFICIENT,
/// )?;
/// assert!((re.value() - 4.2).abs() < 1e-12);
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn effective_rainfall(
    dependable_rainfall: WaterLayer,
    period: Days,
    coefficient: f64,
) -> Result<DepthRate> {
    let c = in_range("rainfall_coefficient", coefficient, 0.0, 1.0)?;
    let days = positive("period", period.value())?;
    DepthRate::new(guard_result(
        "effective_rainfall",
        c * dependable_rainfall.value() / days,
    )?)
}

/// The net field water requirement and the terms that make it up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldRequirement {
    /// Crop water requirement Etc, mm/day.
    pub crop_water_requirement: DepthRate,
    /// Percolation P, mm/day. A soil property, project data.
    pub percolation: DepthRate,
    /// Water layer replacement WLR spread over its period, mm/day.
    pub water_layer_replacement: DepthRate,
    /// Effective rainfall Re, mm/day.
    pub effective_rainfall: DepthRate,
    /// The raw balance `Etc + P + WLR - Re`, mm/day, which may be negative when
    /// rainfall exceeds demand.
    pub balance: f64,
    /// The net field requirement, mm/day, the raw balance floored at zero. A
    /// negative balance means no irrigation water is needed in the period, not
    /// that water flows backwards.
    pub net_field_requirement: DepthRate,
    /// True when the raw balance was negative, that is the period is in
    /// rainfall surplus.
    pub rainfall_surplus: bool,
}

/// Computes the net field water requirement, NFR.
///
/// `NFR = Etc + P + WLR - Re`, all terms in mm/day. During land preparation the
/// `Etc + P` pair is replaced by the land preparation requirement from
/// [`land_preparation_requirement`]; pass that as the crop water requirement
/// and zero percolation in that case, to avoid counting percolation twice.
///
/// # Errors
///
/// Returns [`crate::KpError`] if any term is not finite.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::units::DepthRate;
/// use imaji_kp_hydraulics::water::net_field_requirement;
///
/// // Etc = 5.06, P = 2.00, WLR = 1.67, Re = 4.20 mm/day
/// // NFR = 5.06 + 2.00 + 1.67 - 4.20 = 4.53 mm/day
/// let nfr = net_field_requirement(
///     DepthRate::new(5.06)?,
///     DepthRate::new(2.00)?,
///     DepthRate::new(1.67)?,
///     DepthRate::new(4.20)?,
/// )?;
/// assert!((nfr.net_field_requirement.value() - 4.53).abs() < 1e-12);
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn net_field_requirement(
    crop_water_requirement: DepthRate,
    percolation: DepthRate,
    water_layer_replacement: DepthRate,
    effective_rainfall: DepthRate,
) -> Result<FieldRequirement> {
    let balance = checked_balance(
        "field_balance",
        crop_water_requirement.value() + percolation.value() + water_layer_replacement.value()
            - effective_rainfall.value(),
    )?;
    Ok(FieldRequirement {
        crop_water_requirement,
        percolation,
        water_layer_replacement,
        effective_rainfall,
        balance,
        net_field_requirement: DepthRate::new(balance.max(0.0))?,
        rainfall_surplus: balance < 0.0,
    })
}

/// Land preparation water requirement by the van de Goor and Zijlstra method,
/// as adopted by KP-01.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandPreparation {
    /// `M = Eo + P`, the water lost by evaporation and percolation from a
    /// saturated but not yet cropped field, mm/day.
    pub saturated_field_loss: DepthRate,
    /// The exponent `k = M T / S`, dimensionless.
    pub k: f64,
    /// The land preparation requirement `IR = M e^k / (e^k - 1)`, mm/day.
    pub requirement: DepthRate,
}

/// Land preparation requirement `IR = M e^k / (e^k - 1)`, with `M = Eo + P` and
/// `k = M T / S`.
///
/// `T` is the land preparation period in days and `S` the water depth needed to
/// saturate the soil plus the standing layer to be established, in mm. Both are
/// project data: KP-01 discusses typical values, which depend on how long the
/// land has been fallow.
///
/// `Eo` is open water evaporation during land preparation, usually taken as
/// [`DEFAULT_EO_FACTOR`] times `Eto`, see [`open_water_evaporation`].
///
/// The result already contains percolation through `M`, so do not add
/// percolation again when forming the net field requirement for a land
/// preparation period.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the period or the layer is zero, or if the
/// exponential overflows.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::units::{Days, DepthRate, WaterLayer};
/// use imaji_kp_hydraulics::water::land_preparation_requirement;
///
/// // Eo = 5.5 mm/day, P = 2.0 mm/day, T = 30 days, S = 250 mm
/// // M = 7.5, k = 7.5 × 30 / 250 = 0.9
/// // e^0.9 = 2.459603, IR = 7.5 × 2.459603 / 1.459603 = 12.638 mm/day
/// let lp = land_preparation_requirement(
///     DepthRate::new(5.5)?,
///     DepthRate::new(2.0)?,
///     Days::new(30.0)?,
///     WaterLayer::new(250.0)?,
/// )?;
/// assert!((lp.k - 0.9).abs() < 1e-12);
/// assert!((lp.requirement.value() - 12.638).abs() < 1e-3);
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn land_preparation_requirement(
    open_water_evaporation: DepthRate,
    percolation: DepthRate,
    preparation_period: Days,
    saturation_layer: WaterLayer,
) -> Result<LandPreparation> {
    let m = positive(
        "saturated_field_loss",
        open_water_evaporation.value() + percolation.value(),
    )?;
    let t = positive("preparation_period", preparation_period.value())?;
    let s = positive("saturation_layer", saturation_layer.value())?;
    let k = guard_result("k", m * t / s)?;
    let exp_k = guard_result("exp_k", k.exp())?;
    let denominator = positive("exp_k_minus_one", exp_k - 1.0)?;
    Ok(LandPreparation {
        saturated_field_loss: DepthRate::new(m)?,
        k,
        requirement: DepthRate::new(guard_result("land_preparation", m * exp_k / denominator)?)?,
    })
}

/// Open water evaporation during land preparation, `Eo = factor × Eto`.
///
/// # Errors
///
/// Returns [`crate::KpError`] if the factor is negative or the result is not
/// finite.
pub fn open_water_evaporation(reference_et: DepthRate, factor: f64) -> Result<DepthRate> {
    let factor = non_negative("eo_factor", factor)?;
    DepthRate::new(guard_result(
        "open_water_evaporation",
        factor * reference_et.value(),
    )?)
}

/// The three stage irrigation efficiency chain of KP-01.
///
/// Efficiency is the fraction of water entering a level that reaches the next
/// level down. Losses are conveyance and operational losses, not consumptive
/// use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EfficiencyChain {
    /// Tertiary efficiency, from the tertiary offtake to the field.
    pub tertiary: Ratio,
    /// Secondary efficiency, from the secondary canal to the tertiary offtake.
    pub secondary: Ratio,
    /// Primary efficiency, from the intake to the secondary canal.
    pub primary: Ratio,
}

impl EfficiencyChain {
    /// Builds a chain, rejecting any efficiency outside the open interval 0 to
    /// 1.
    ///
    /// An efficiency of exactly 1 is rejected as well as 0: a lossless canal is
    /// not a physical design assumption, and accepting it silently hides an
    /// input error.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError::OutOfRange`] if any efficiency is not strictly
    /// between 0 and 1.
    pub fn new(tertiary: f64, secondary: f64, primary: f64) -> Result<Self> {
        let check = |name: &'static str, v: f64| -> Result<Ratio> {
            let v = in_range(name, v, 0.0, 1.0)?;
            Ratio::new(positive(name, v)?)
        };
        Ok(Self {
            tertiary: check("tertiary_efficiency", tertiary)?,
            secondary: check("secondary_efficiency", secondary)?,
            primary: check("primary_efficiency", primary)?,
        })
    }

    /// The overall efficiency, the product of the three stages.
    pub fn overall(&self) -> f64 {
        self.tertiary.value() * self.secondary.value() * self.primary.value()
    }
}

impl Default for EfficiencyChain {
    /// A convenience default of 0.80 tertiary, 0.90 secondary, 0.90 primary,
    /// giving 0.648 overall.
    ///
    /// These are widely published representative figures for an Indonesian
    /// technical irrigation scheme, provided so that examples run without
    /// project data to hand. Confirm them against the current KP-01 edition and
    /// against measured scheme performance before using a design.
    fn default() -> Self {
        Self {
            tertiary: Ratio::from_valid(0.80),
            secondary: Ratio::from_valid(0.90),
            primary: Ratio::from_valid(0.90),
        }
    }
}

/// The requirement carried up the chain from the field to the intake.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiversionRequirement {
    /// Irrigated area served, ha.
    pub area: Hectares,
    /// Net field requirement, mm/day.
    pub net_field_requirement: DepthRate,
    /// Requirement at the tertiary offtake, NFR / e_tertiary, mm/day.
    pub at_tertiary_offtake: DepthRate,
    /// Requirement at the secondary canal head, mm/day.
    pub at_secondary_head: DepthRate,
    /// Requirement at the intake, the diversion requirement DR, mm/day.
    pub at_intake: DepthRate,
    /// The efficiency chain used.
    pub efficiency: EfficiencyChain,
    /// Overall efficiency, the product of the three stages.
    pub overall_efficiency: f64,
    /// Unit diversion requirement at the intake, l/s per ha.
    pub unit_discharge_lps_per_ha: f64,
    /// Discharge to be diverted at the intake, m³/s.
    pub intake_discharge: Discharge,
    /// Discharge required at the tertiary offtake for the same area, m³/s.
    pub tertiary_offtake_discharge: Discharge,
}

impl fmt::Display for DiversionRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Diversion requirement, KP-01 efficiency chain")?;
        writeln!(
            f,
            "  irrigated area              {:>10.1} ha",
            self.area.value()
        )?;
        writeln!(
            f,
            "  net field requirement NFR   {:>10.3} mm/day",
            self.net_field_requirement.value()
        )?;
        writeln!(
            f,
            "  tertiary efficiency         {:>10.3}",
            self.efficiency.tertiary.value()
        )?;
        writeln!(
            f,
            "  at tertiary offtake         {:>10.3} mm/day",
            self.at_tertiary_offtake.value()
        )?;
        writeln!(
            f,
            "  secondary efficiency        {:>10.3}",
            self.efficiency.secondary.value()
        )?;
        writeln!(
            f,
            "  at secondary head           {:>10.3} mm/day",
            self.at_secondary_head.value()
        )?;
        writeln!(
            f,
            "  primary efficiency          {:>10.3}",
            self.efficiency.primary.value()
        )?;
        writeln!(
            f,
            "  diversion requirement DR    {:>10.3} mm/day",
            self.at_intake.value()
        )?;
        writeln!(
            f,
            "  overall efficiency          {:>10.3}",
            self.overall_efficiency
        )?;
        writeln!(
            f,
            "  unit discharge at intake    {:>10.3} l/s/ha",
            self.unit_discharge_lps_per_ha
        )?;
        writeln!(
            f,
            "  intake discharge            {:>10.3} m³/s ({:.1} l/s)",
            self.intake_discharge.value(),
            self.intake_discharge.as_litres_per_second()
        )?;
        write!(
            f,
            "  tertiary offtake discharge  {:>10.3} m³/s ({:.1} l/s)",
            self.tertiary_offtake_discharge.value(),
            self.tertiary_offtake_discharge.as_litres_per_second()
        )
    }
}

/// Carries a net field requirement up the efficiency chain to the intake.
///
/// Each stage divides by its efficiency, so the requirement grows going
/// upstream. The intake discharge is then `DR × A / 8.64` in l/s, handled by
/// [`DepthRate::over_area`].
///
/// # Errors
///
/// Returns [`crate::KpError`] if the area is zero or a division produces a non
/// finite value.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
/// The efficiencies are scheme specific and the result must be checked against
/// the governing criteria and the scheme water balance.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::units::{DepthRate, Hectares};
/// use imaji_kp_hydraulics::water::{diversion_requirement, EfficiencyChain};
///
/// // NFR = 1.30 mm/day, efficiencies 0.80 / 0.90 / 0.90, area 1 250 ha
/// // At tertiary offtake: 1.30 / 0.80 = 1.625 mm/day
/// // At secondary head:   1.625 / 0.90 = 1.80556 mm/day
/// // At intake:           1.80556 / 0.90 = 2.00617 mm/day
/// // Unit discharge:      2.00617 / 8.64 = 0.232196 l/s/ha
/// // Intake discharge:    0.232196 × 1250 = 290.25 l/s
/// let dr = diversion_requirement(
///     DepthRate::new(1.30)?,
///     EfficiencyChain::new(0.80, 0.90, 0.90)?,
///     Hectares::new(1250.0)?,
/// )?;
/// assert!((dr.at_intake.value() - 2.006_17).abs() < 1e-5);
/// assert!((dr.intake_discharge.as_litres_per_second() - 290.25).abs() < 1e-2);
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn diversion_requirement(
    net_field_requirement: DepthRate,
    efficiency: EfficiencyChain,
    area: Hectares,
) -> Result<DiversionRequirement> {
    positive("area", area.value())?;
    let nfr = net_field_requirement.value();
    let at_tertiary = DepthRate::new(guard_result(
        "at_tertiary_offtake",
        nfr / efficiency.tertiary.value(),
    )?)?;
    let at_secondary = DepthRate::new(guard_result(
        "at_secondary_head",
        at_tertiary.value() / efficiency.secondary.value(),
    )?)?;
    let at_intake = DepthRate::new(guard_result(
        "at_intake",
        at_secondary.value() / efficiency.primary.value(),
    )?)?;

    Ok(DiversionRequirement {
        area,
        net_field_requirement,
        at_tertiary_offtake: at_tertiary,
        at_secondary_head: at_secondary,
        at_intake,
        efficiency,
        overall_efficiency: efficiency.overall(),
        unit_discharge_lps_per_ha: at_intake.as_litres_per_second_per_hectare(),
        intake_discharge: at_intake.over_area(area)?,
        tertiary_offtake_discharge: at_tertiary.over_area(area)?,
    })
}

/// How the available supply is to be delivered when it falls short of demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupplyMode {
    /// Supply meets demand closely enough to run every tertiary block
    /// continuously.
    Continuous,
    /// Supply is short, so tertiary blocks take turns in the given number of
    /// groups (golongan).
    Rotation {
        /// Number of groups the blocks are split into.
        groups: u8,
    },
    /// Supply is so short that rotation will not save the planned cropping.
    /// The cropping pattern or the planted area has to be replanned.
    Insufficient,
}

impl fmt::Display for SupplyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SupplyMode::Continuous => f.write_str("continuous supply"),
            SupplyMode::Rotation { groups } => write!(f, "rotation among {groups} groups"),
            SupplyMode::Insufficient => {
                f.write_str("supply insufficient, replan the cropping pattern")
            }
        }
    }
}

/// Thresholds on the k factor that decide the supply mode.
///
/// KP-01 reduces the delivered area, or moves to rotational supply, when the
/// available discharge falls below the requirement. The exact thresholds are an
/// operational decision for the scheme, so they are caller input here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotationCriteria {
    /// At or above this k factor, supply continuously.
    pub continuous_above: f64,
    /// At or above this k factor but below `continuous_above`, rotate in two
    /// groups.
    pub two_groups_above: f64,
    /// At or above this k factor but below `two_groups_above`, rotate in three
    /// groups. Below it, the supply is treated as insufficient.
    pub three_groups_above: f64,
}

impl Default for RotationCriteria {
    /// A convenience default of 0.80, 0.60 and 0.40.
    ///
    /// These thresholds are a documented assumption of this crate, not a
    /// quotation from KP-01, which discusses rotation qualitatively and leaves
    /// the trigger to the scheme operator. Set them to the project's own
    /// operating rule.
    fn default() -> Self {
        Self {
            continuous_above: 0.80,
            two_groups_above: 0.60,
            three_groups_above: 0.40,
        }
    }
}

/// Assessment of a short supply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShortSupply {
    /// Discharge available at the intake, m³/s.
    pub available: Discharge,
    /// Discharge required at the intake, m³/s.
    pub required: Discharge,
    /// The k factor, available over required, capped at 1.
    pub k_factor: f64,
    /// The area that can be irrigated at full duty with the available supply,
    /// ha, that is `k × planned area`. This is the relative area method.
    pub relative_area: Hectares,
    /// The planned area, ha.
    pub planned_area: Hectares,
    /// The supply mode indicated by the k factor.
    pub mode: SupplyMode,
}

/// Assesses a supply that may be short of the requirement.
///
/// The k factor is the ratio of available to required discharge, capped at 1
/// because a surplus does not increase the duty. The relative area is the area
/// that the available supply can serve at full duty, `k × planned area`, which
/// is how KP-01 reduces the served area when the source cannot meet the whole
/// scheme. The supply mode comes from [`RotationCriteria`].
///
/// # Errors
///
/// Returns [`crate::KpError::NotPositive`] if the required discharge or the
/// planned area is zero.
///
/// # Disclaimer
///
/// A calculation aid, not a substitute for a licensed engineer's judgement.
///
/// # Examples
///
/// ```
/// use imaji_kp_hydraulics::units::{Discharge, Hectares};
/// use imaji_kp_hydraulics::water::{assess_short_supply, RotationCriteria, SupplyMode};
///
/// // Available 0.42 m³/s against a requirement of 0.60 m³/s: k = 0.70
/// // Relative area = 0.70 × 800 ha = 560 ha, and k = 0.70 calls for
/// // rotation in two groups under the default criteria.
/// let s = assess_short_supply(
///     Discharge::new(0.42)?,
///     Discharge::new(0.60)?,
///     Hectares::new(800.0)?,
///     RotationCriteria::default(),
/// )?;
/// assert!((s.k_factor - 0.70).abs() < 1e-12);
/// assert!((s.relative_area.value() - 560.0).abs() < 1e-9);
/// assert_eq!(s.mode, SupplyMode::Rotation { groups: 2 });
/// # Ok::<(), imaji_kp_hydraulics::KpError>(())
/// ```
pub fn assess_short_supply(
    available: Discharge,
    required: Discharge,
    planned_area: Hectares,
    criteria: RotationCriteria,
) -> Result<ShortSupply> {
    let required_value = positive("required_discharge", required.value())?;
    positive("planned_area", planned_area.value())?;
    let k = guard_result("k_factor", available.value() / required_value)?.min(1.0);
    let mode = if k >= criteria.continuous_above {
        SupplyMode::Continuous
    } else if k >= criteria.two_groups_above {
        SupplyMode::Rotation { groups: 2 }
    } else if k >= criteria.three_groups_above {
        SupplyMode::Rotation { groups: 3 }
    } else {
        SupplyMode::Insufficient
    };
    Ok(ShortSupply {
        available,
        required,
        k_factor: k,
        relative_area: Hectares::new(guard_result("relative_area", k * planned_area.value())?)?,
        planned_area,
        mode,
    })
}
