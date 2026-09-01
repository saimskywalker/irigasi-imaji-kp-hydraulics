#![deny(missing_docs)]
//! Irrigation design calculations following the Indonesian Kriteria
//! Perencanaan (KP) irrigation planning criteria, KP-01 to KP-07, published by
//! the Direktorat Jenderal Sumber Daya Air, Kementerian Pekerjaan Umum dan
//! Perumahan Rakyat.
//!
//! The crate implements the equations and the procedures and cites the KP part
//! each one belongs to. It does not reproduce the text of the criteria, and it
//! does not embed their design tables. Where a criterion is a value the
//! engineer reads off a chart or a table, such as freeboard against discharge,
//! minimum side slope against soil type, or maximum permissible velocity
//! against bed material, the value is a parameter of the function or a
//! caller-supplied table. A few types carry a `Default` with a handful of
//! widely published representative values, so that examples run without project
//! data to hand; each of those is documented as a convenience the engineer must
//! confirm against the current KP edition.
//!
//! # Disclaimer
//!
//! Everything here is a calculation aid. It is not a substitute for a licensed
//! engineer's judgement, and no result from it should be built without being
//! checked against the governing criteria and against the conditions of the
//! site.
//!
//! # Unit convention
//!
//! SI throughout, with the two exceptions that Indonesian irrigation practice
//! insists on: depth rates in mm/day and areas in hectares. Every quantity is a
//! distinct newtype, so the compiler rejects a discharge passed where a depth
//! rate is wanted, and every conversion is an explicitly named method. See
//! [`units`] for the full table.
//!
//! | Quantity | Type | Unit |
//! |---|---|---|
//! | Length, depth, head | [`units::Length`] | m |
//! | Area | [`units::Area`] | m² |
//! | Velocity | [`units::Velocity`] | m/s |
//! | Discharge | [`units::Discharge`] | m³/s |
//! | Slope | [`units::Slope`] | m/m |
//! | Side slope | [`units::SideSlope`] | m horizontal per m vertical |
//! | Manning roughness | [`units::ManningN`] | s·m^(-1/3) |
//! | Evapotranspiration, rainfall, percolation | [`units::DepthRate`] | mm/day |
//! | Standing water layer | [`units::WaterLayer`] | mm |
//! | Irrigated area | [`units::Hectares`] | ha |
//!
//! # Modules
//!
//! - [`units`], [`error`], [`check`]: the foundations. Units, the single error
//!   type, and the design check record that every design routine returns.
//! - [`section`]: trapezoidal, rectangular and triangular cross section
//!   geometry.
//! - [`uniform_flow`]: Manning's equation both ways, normal depth by bisection,
//!   Froude number, flow regime, velocity limits. KP-03.
//! - [`freeboard`]: freeboard rules as a caller-supplied table or function.
//!   KP-03.
//! - [`design`]: the most efficient trapezoid, and the full canal design
//!   routine with all its checks. KP-03.
//! - [`water`]: net field requirement, land preparation, the efficiency chain
//!   to the intake, and short supply assessment. KP-01.
//! - [`tertiary`]: tertiary and quaternary block sizing and layout limits.
//!   KP-01 and KP-07.
//! - [`structures`]: weir, orifice and gate discharge relations, measuring
//!   weirs, hydraulic jump and straight drop energy dissipation. KP-02 and
//!   KP-04.
//! - [`conduit`]: culvert and siphon head loss and barrel sizing. KP-04.
//!
//! # No panics
//!
//! Library code never panics on bad input. Every fallible operation returns
//! [`Result`] with a descriptive [`KpError`]. Iterative solvers are bounded and
//! return [`KpError::DidNotConverge`] or [`KpError::NoBracket`] rather than
//! looping or returning an unconverged answer.
//!
//! # Example
//!
//! Sizing a secondary canal for 1.5 m³/s and reading the checks:
//!
//! ```
//! use kp_irigasi::design::{design_channel, ChannelDesignInput, SectionProportion};
//! use kp_irigasi::freeboard::FreeboardTable;
//! use kp_irigasi::uniform_flow::{NormalDepthOptions, VelocityLimits};
//! use kp_irigasi::units::{Discharge, ManningN, SideSlope, Slope};
//!
//! let freeboard = FreeboardTable::default();
//! let design = design_channel(&ChannelDesignInput {
//!     design_discharge: Discharge::new(1.5)?,
//!     bed_slope: Slope::one_in(2000.0)?,
//!     side_slope: SideSlope::new(1.5)?,
//!     roughness: ManningN::new(0.025)?,
//!     proportion: SectionProportion::BedWidthDepthRatio(2.0),
//!     velocity_limits: VelocityLimits::from_range(0.25, 0.70)?,
//!     freeboard_rule: &freeboard,
//!     max_froude: Some(0.7),
//!     min_freeboard: None,
//!     solver: NormalDepthOptions::default(),
//! })?;
//!
//! assert!(design.all_checks_pass());
//! println!("{design}");
//! # Ok::<(), kp_irigasi::KpError>(())
//! ```

pub mod check;
pub mod conduit;
pub mod design;
pub mod error;
pub mod freeboard;
pub mod section;
pub mod structures;
pub mod tertiary;
pub mod uniform_flow;
pub mod units;
pub mod water;

pub use error::{KpError, Result};

/// Acceleration due to gravity, m/s².
///
/// The value 9.81 m/s² is the one Indonesian irrigation practice works to. It
/// is used in every Froude number, weir, orifice and energy calculation in the
/// crate, so a result can be reproduced by a hand calculation using the same
/// figure.
pub const GRAVITY: f64 = 9.81;
