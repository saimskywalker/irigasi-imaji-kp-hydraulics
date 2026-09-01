//! Sizes a straight drop basin and a siphon barrel, and prints both reports.
//!
//! Run with:
//!
//! ```text
//! cargo run -p kp-irigasi --example drop_and_siphon
//! ```
//!
//! The loss coefficients and the permissible velocities are project data read
//! from KP-04. This example is a calculation aid, not a design.

use kp_irigasi::conduit::{
    conduit_head_loss, size_circular_barrel, Barrel, BarrelSizingOptions, ConduitInput,
    LossCoefficients,
};
use kp_irigasi::structures::{cipoletti_weir_discharge, straight_drop, DEFAULT_JUMP_LENGTH_FACTOR};
use kp_irigasi::uniform_flow::VelocityLimits;
use kp_irigasi::units::{Discharge, Length, ManningN};
use kp_irigasi::KpError;

fn main() -> Result<(), KpError> {
    // --- Straight drop on a secondary canal --------------------------------
    let discharge = Discharge::new(1.20)?;
    let crest_width = Length::new(2.00)?;
    let unit_discharge = discharge.value() / crest_width.value();
    let drop = straight_drop(
        unit_discharge,
        Length::new(1.25)?,
        DEFAULT_JUMP_LENGTH_FACTOR,
    )?;

    println!("Bangunan terjun (drop structure) BT.3 on secondary canal SS.1");
    println!(
        "Q = {:.2} m³/s over a {:.2} m crest, q = {:.4} m³/s per m\n",
        discharge.value(),
        crest_width.value(),
        unit_discharge
    );
    println!("{drop}");
    println!(
        "\n  Basin floor set {:.3} m below the downstream bed and {:.2} m long,\n\
         \x20 before rounding and before the cut off walls, the nappe ventilation\n\
         \x20 and the downstream protection are designed.",
        drop.sequent_depth.value(),
        drop.basin_length.value()
    );

    // --- Siphon under a road crossing --------------------------------------
    let siphon_discharge = Discharge::new(0.85)?;
    let barrel_length = Length::new(32.0)?;
    let roughness = ManningN::new(0.014)?; // concrete pipe
                                           // Entry 0.5, exit 1.0, two elbows at 0.20 each.
    let coefficients = LossCoefficients::new(0.5, 1.0, 0.40)?;
    let velocity_limits = VelocityLimits::from_range(1.0, 3.0)?;

    let sizing = size_circular_barrel(
        siphon_discharge,
        barrel_length,
        roughness,
        coefficients,
        Length::new(0.25)?,
        velocity_limits,
        BarrelSizingOptions::default(),
    )?;

    println!("\n\nSiphon BS.2 under the district road, allowable head loss 0.25 m");
    println!(
        "  solver: bisection on diameter, {} iterations, tolerance {:.0e} m",
        sizing.iterations,
        BarrelSizingOptions::default().tolerance
    );
    println!(
        "  smallest diameter within the allowance: {:.4} m\n",
        sizing.diameter.value()
    );
    println!("{}", sizing.head_loss);

    // Round up to a manufactured size and recheck, which is what actually gets
    // built.
    let rounded = Length::new(1.00)?;
    let checked = conduit_head_loss(&ConduitInput {
        barrel: Barrel::Circular { diameter: rounded },
        discharge: siphon_discharge,
        length: barrel_length,
        roughness,
        coefficients,
        velocity_limits,
    })?;
    println!(
        "\n\nRounded up to a {:.2} m manufactured pipe and rechecked",
        rounded.value()
    );
    println!("{checked}");

    // --- Measuring weir at the tertiary offtake -----------------------------
    let head = Length::new(0.22)?;
    let measured = cipoletti_weir_discharge(Length::new(0.75)?, head)?;
    println!(
        "\n\nCipoletti measuring weir at the tertiary offtake, b = 0.75 m,\n\
         head {:.2} m: Q = {:.1} l/s",
        head.value(),
        measured.as_litres_per_second()
    );

    println!(
        "\nEvery result above is a calculation aid. It is not a substitute for a\n\
         licensed engineer's judgement and must be checked against the governing\n\
         Kriteria Perencanaan."
    );

    Ok(())
}
