//! Explicit units for every quantity the crate handles.
//!
//! # Unit convention
//!
//! The crate is SI throughout, with two deliberate exceptions that follow
//! Indonesian irrigation practice:
//!
//! | Quantity | Type | Unit |
//! |---|---|---|
//! | Length, depth, width, head | [`Length`] | m |
//! | Flow area, cross section area | [`Area`] | m² |
//! | Velocity | [`Velocity`] | m/s |
//! | Discharge | [`Discharge`] | m³/s |
//! | Bed slope, energy slope | [`Slope`] | m/m |
//! | Side slope | [`SideSlope`] | m horizontal per 1 m vertical |
//! | Manning roughness | [`ManningN`] | s·m^(-1/3) |
//! | Dimensionless ratio | [`Ratio`] | none |
//! | Evapotranspiration, percolation, rainfall | [`DepthRate`] | mm/day |
//! | Standing water layer | [`WaterLayer`] | mm |
//! | Irrigated area | [`Hectares`] | ha |
//! | Period | [`Days`] | day |
//!
//! Irrigation practice mixes litres per second, m³/s and mm/day freely, and a
//! silent unit mix-up is the classic way to get an irrigation calculation
//! wrong. Every quantity here is a distinct type, so the compiler refuses a
//! discharge where a depth rate is wanted. Conversions are explicit and named:
//! [`Discharge::from_litres_per_second`], [`DepthRate::over_area`],
//! [`Hectares::as_square_metres`].
//!
//! Every constructor validates its argument. `NaN`, infinity and negative
//! values for quantities that cannot be negative are rejected with a
//! [`KpError`], so an invalid number cannot travel silently into a design.

use crate::error::{finite, non_negative, positive, KpError, Result};
use std::fmt;

/// Seconds in a day, used for the mm/day to m³/s conversions.
pub const SECONDS_PER_DAY: f64 = 86_400.0;

/// Square metres in a hectare.
pub const SQUARE_METRES_PER_HECTARE: f64 = 10_000.0;

/// Conversion divisor from mm/day to litres per second per hectare.
///
/// 1 mm/day spread over 1 ha is 0.001 m × 10 000 m² = 10 m³/day, and
/// 10 000 litres over 86 400 s is 0.11574 l/s, that is 1/8.64.
pub const MM_PER_DAY_TO_LPS_PER_HA: f64 = 8.64;

macro_rules! quantity {
    (
        $(#[$attr:meta])*
        $name:ident, unit = $unit:literal, guard = $guard:ident
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name(f64);

        impl $name {
            #[doc = concat!("The unit symbol, `", $unit, "`.")]
            pub const UNIT: &'static str = $unit;

            #[doc = concat!("Creates a value in ", $unit, ".")]
            ///
            /// # Errors
            ///
            /// Returns [`KpError`] if the value is `NaN`, infinite, or outside
            /// the sign the quantity permits.
            pub fn new(value: f64) -> Result<Self> {
                Ok(Self($guard(stringify!($name), value)?))
            }

            #[doc = concat!("Returns the underlying magnitude in ", $unit, ".")]
            pub const fn value(self) -> f64 {
                self.0
            }

            /// Builds a value from a literal that is known at compile time to
            /// be valid, skipping validation.
            ///
            /// Internal to the crate, used only for the documented convenience
            /// defaults so that they need no `unwrap`. Every value arriving
            /// from outside the crate goes through `new` and is validated.
            #[allow(dead_code)]
            pub(crate) const fn from_valid(value: f64) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)?;
                write!(f, " {}", $unit)
            }
        }
    };
}

quantity! {
    /// A length in metres: a flow depth, a bed width, a head, a freeboard.
    ///
    /// Must not be negative. Zero is allowed, because a rectangular section is
    /// a trapezoid with zero side slope and a triangular section is one with
    /// zero bed width. Routines that cannot work with a zero length, such as
    /// Manning's equation at zero depth, reject it themselves.
    Length, unit = "m", guard = non_negative
}

quantity! {
    /// A flow area or cross section area in square metres.
    Area, unit = "m²", guard = non_negative
}

quantity! {
    /// A mean flow velocity in metres per second.
    Velocity, unit = "m/s", guard = non_negative
}

quantity! {
    /// A discharge in cubic metres per second.
    ///
    /// Use [`Discharge::from_litres_per_second`] when the figure to hand is in
    /// l/s, which is usual for tertiary block and measuring structure work.
    Discharge, unit = "m³/s", guard = non_negative
}

quantity! {
    /// A longitudinal slope in metres fall per metre run, dimensionless.
    ///
    /// Zero is accepted by the constructor because a level bed is a real
    /// geometry, but uniform flow routines reject it: Manning's equation has no
    /// solution at zero energy slope.
    Slope, unit = "m/m", guard = non_negative
}

quantity! {
    /// A side slope, metres horizontal per one metre vertical.
    ///
    /// A value of 1.5 means 1 vertical to 1.5 horizontal, written 1:1.5 in
    /// Indonesian practice. Zero means a vertical wall. The minimum side slope
    /// against soil type is a KP-03 lookup and is not embedded here: pass the
    /// value the criteria give for the soil at hand.
    SideSlope, unit = "m/m", guard = non_negative
}

quantity! {
    /// Manning's roughness coefficient n, in s·m^(-1/3).
    ///
    /// The value against lining type and channel size is a KP-03 lookup and is
    /// not embedded here. Pass the value the criteria give for the channel at
    /// hand.
    ManningN, unit = "s·m^(-1/3)", guard = positive
}

quantity! {
    /// A dimensionless non-negative ratio: an efficiency, a coefficient, a
    /// crop factor.
    Ratio, unit = "", guard = non_negative
}

quantity! {
    /// A depth rate in millimetres per day.
    ///
    /// This is the unit KP-01 works in for evapotranspiration, percolation,
    /// rainfall and the net field requirement. It is deliberately not a
    /// [`Velocity`]: converting it to a discharge needs an area, which
    /// [`DepthRate::over_area`] makes explicit.
    DepthRate, unit = "mm/day", guard = non_negative
}

quantity! {
    /// A standing water depth in millimetres.
    ///
    /// Used for the puddling and saturation layer in land preparation, for the
    /// water layer replacement, and for a rainfall total over a period.
    WaterLayer, unit = "mm", guard = non_negative
}

quantity! {
    /// An irrigated area in hectares.
    ///
    /// Kept distinct from [`Area`] in m², because irrigation areas are always
    /// quoted in hectares and a factor of 10 000 error here is silent and
    /// serious.
    Hectares, unit = "ha", guard = non_negative
}

quantity! {
    /// A period in days.
    Days, unit = "day", guard = positive
}

impl Length {
    /// Creates a length from a value in millimetres.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the value is not finite or is negative.
    pub fn from_millimetres(millimetres: f64) -> Result<Self> {
        Self::new(non_negative("millimetres", millimetres)? / 1000.0)
    }

    /// Returns the length in millimetres.
    pub fn as_millimetres(self) -> f64 {
        self.0 * 1000.0
    }
}

impl Discharge {
    /// Creates a discharge from a value in litres per second.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the value is not finite or is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use imaji_kp_hydraulics::units::Discharge;
    ///
    /// let q = Discharge::from_litres_per_second(250.0)?;
    /// assert!((q.value() - 0.25).abs() < 1e-12);
    /// # Ok::<(), imaji_kp_hydraulics::KpError>(())
    /// ```
    pub fn from_litres_per_second(litres_per_second: f64) -> Result<Self> {
        Self::new(non_negative("litres_per_second", litres_per_second)? / 1000.0)
    }

    /// Returns the discharge in litres per second.
    pub fn as_litres_per_second(self) -> f64 {
        self.0 * 1000.0
    }

    /// Expresses this discharge as a depth rate over an irrigated area.
    ///
    /// This is the inverse of [`DepthRate::over_area`]. Useful when comparing
    /// an available intake discharge against a demand quoted in mm/day.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the area is zero.
    pub fn as_depth_rate_over(self, area: Hectares) -> Result<DepthRate> {
        let hectares = positive("area", area.value())?;
        DepthRate::new(self.0 * MM_PER_DAY_TO_LPS_PER_HA * 1000.0 / hectares)
    }
}

impl DepthRate {
    /// Returns the depth rate as litres per second per hectare.
    ///
    /// Divides by 8.64, see [`MM_PER_DAY_TO_LPS_PER_HA`].
    ///
    /// # Examples
    ///
    /// ```
    /// use imaji_kp_hydraulics::units::DepthRate;
    ///
    /// // 1 mm/day over 1 ha is 10 m³/day, that is 0.11574 l/s.
    /// let unit = DepthRate::new(1.0)?.as_litres_per_second_per_hectare();
    /// assert!((unit - 0.115_740_74).abs() < 1e-8);
    /// # Ok::<(), imaji_kp_hydraulics::KpError>(())
    /// ```
    pub fn as_litres_per_second_per_hectare(self) -> f64 {
        self.0 / MM_PER_DAY_TO_LPS_PER_HA
    }

    /// Converts a depth rate over an irrigated area into a discharge.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the resulting discharge is not representable,
    /// which in practice cannot happen for finite inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use imaji_kp_hydraulics::units::{DepthRate, Hectares};
    ///
    /// // 1.2 mm/day over 500 ha: 1.2 / 8.64 = 0.13889 l/s/ha, times 500 ha
    /// // is 69.44 l/s.
    /// let q = DepthRate::new(1.2)?.over_area(Hectares::new(500.0)?)?;
    /// assert!((q.as_litres_per_second() - 69.444_44).abs() < 1e-4);
    /// # Ok::<(), imaji_kp_hydraulics::KpError>(())
    /// ```
    pub fn over_area(self, area: Hectares) -> Result<Discharge> {
        Discharge::new(self.as_litres_per_second_per_hectare() * area.value() / 1000.0)
    }

    /// Returns the equivalent depth accumulated over a period, in millimetres.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the result is not finite.
    pub fn over_period(self, days: Days) -> Result<WaterLayer> {
        WaterLayer::new(self.0 * days.value())
    }
}

impl WaterLayer {
    /// Spreads a standing water depth evenly over a period, giving a depth rate.
    ///
    /// This is how KP-01 turns a water layer replacement, for example 50 mm
    /// applied twice in a season, into an mm/day term in the field requirement.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the result is not finite.
    ///
    /// # Examples
    ///
    /// ```
    /// use imaji_kp_hydraulics::units::{Days, WaterLayer};
    ///
    /// // 50 mm spread over 30 days is 1.667 mm/day.
    /// let wlr = WaterLayer::new(50.0)?.spread_over(Days::new(30.0)?)?;
    /// assert!((wlr.value() - 1.666_667).abs() < 1e-6);
    /// # Ok::<(), imaji_kp_hydraulics::KpError>(())
    /// ```
    pub fn spread_over(self, days: Days) -> Result<DepthRate> {
        DepthRate::new(self.0 / days.value())
    }

    /// Returns the water layer in metres.
    pub fn as_metres(self) -> f64 {
        self.0 / 1000.0
    }
}

impl Hectares {
    /// Returns the area in square metres.
    pub fn as_square_metres(self) -> f64 {
        self.0 * SQUARE_METRES_PER_HECTARE
    }

    /// Creates an area in hectares from a value in square metres.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the value is not finite or is negative.
    pub fn from_square_metres(square_metres: f64) -> Result<Self> {
        Self::new(non_negative("square_metres", square_metres)? / SQUARE_METRES_PER_HECTARE)
    }
}

impl Ratio {
    /// Creates a ratio that must lie between 0 and 1 inclusive, such as an
    /// irrigation efficiency.
    ///
    /// # Errors
    ///
    /// Returns [`KpError::OutOfRange`] if the value is outside 0 to 1.
    pub fn fraction(value: f64) -> Result<Self> {
        Ok(Self(crate::error::in_range("fraction", value, 0.0, 1.0)?))
    }
}

impl Slope {
    /// Creates a slope from a fall over a run, for example 0.35 m over 1400 m.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the run is zero, or either value is not finite or
    /// is negative.
    pub fn from_fall_over_run(fall: Length, run: Length) -> Result<Self> {
        let run = positive("run", run.value())?;
        Self::new(fall.value() / run)
    }

    /// Creates a slope from a denominator, so `Slope::one_in(1400.0)` is
    /// 1 in 1400, that is 0.000714 m/m.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the denominator is zero, negative or not finite.
    pub fn one_in(denominator: f64) -> Result<Self> {
        Self::new(1.0 / positive("denominator", denominator)?)
    }
}

impl SideSlope {
    /// Creates a side slope from the horizontal and vertical legs of the batter,
    /// so `SideSlope::from_run_and_rise(1.5, 1.0)` is 1 vertical to 1.5
    /// horizontal.
    ///
    /// # Errors
    ///
    /// Returns [`KpError`] if the rise is zero, or either value is invalid.
    pub fn from_run_and_rise(run: f64, rise: f64) -> Result<Self> {
        let rise = positive("rise", rise)?;
        Self::new(non_negative("run", run)? / rise)
    }

    /// Returns `sqrt(1 + m²)`, the batter length per unit depth, which appears
    /// in every trapezoidal wetted perimeter.
    pub fn batter_factor(self) -> f64 {
        (1.0 + self.0 * self.0).sqrt()
    }
}

impl Days {
    /// Returns the period in seconds.
    pub fn as_seconds(self) -> f64 {
        self.0 * SECONDS_PER_DAY
    }
}

/// Helper used by result types that need to report a value that may legitimately
/// be negative, such as a field water balance where rainfall exceeds demand.
///
/// # Errors
///
/// Returns [`KpError`] if the value is not finite.
pub(crate) fn checked_balance(parameter: &'static str, value: f64) -> Result<f64> {
    finite(parameter, value)
}

/// Guards against a `NaN` sneaking out of a computation, for internal use.
pub(crate) fn guard_result(parameter: &'static str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(KpError::NotFinite { parameter, value })
    }
}
