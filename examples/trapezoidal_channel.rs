//! Designs a trapezoidal secondary canal and prints a calculation report.
//!
//! Run with:
//!
//! ```text
//! cargo run -p imaji-kp-hydraulics --example trapezoidal_channel
//! ```
//!
//! The figures used here are illustrative. The freeboard rule, the roughness,
//! the side slope and the permissible velocities are all project data that the
//! engineer reads off the governing Kriteria Perencanaan. This example is a
//! calculation aid, not a design.

use imaji_kp_hydraulics::design::{design_channel, ChannelDesignInput, SectionProportion};
use imaji_kp_hydraulics::freeboard::FreeboardTable;
use imaji_kp_hydraulics::uniform_flow::{NormalDepthOptions, VelocityLimits};
use imaji_kp_hydraulics::units::{Discharge, Length, ManningN, SideSlope, Slope};
use imaji_kp_hydraulics::KpError;

fn main() -> Result<(), KpError> {
    // Saluran sekunder (secondary canal), unlined, in a stable clay loam.
    let design_discharge = Discharge::new(1.85)?;
    let bed_slope = Slope::from_fall_over_run(Length::new(0.62)?, Length::new(1550.0)?)?;
    let side_slope = SideSlope::new(1.5)?; // 1 vertical to 1.5 horizontal
    let roughness = ManningN::new(0.025)?; // KP-03 value for an earth canal
    let freeboard = FreeboardTable::default(); // convenience default, confirm against KP-03
    let velocity_limits = VelocityLimits::from_range(0.25, 0.70)?;

    println!("Secondary canal SS.1, chainage 0+000 to 1+550");
    println!("Design basis: KP-03 uniform flow, Manning, trapezoidal section");
    println!(
        "Bed slope from a fall of 0.62 m over 1 550 m: S = {:.6} m/m\n",
        bed_slope.value()
    );

    // First pass: the most hydraulically efficient section for this side slope.
    let efficient = design_channel(&ChannelDesignInput {
        design_discharge,
        bed_slope,
        side_slope,
        roughness,
        proportion: SectionProportion::MostEfficient,
        velocity_limits,
        freeboard_rule: &freeboard,
        max_froude: Some(0.7),
        min_freeboard: None,
        solver: NormalDepthOptions::default(),
    })?;

    println!("Option A, most hydraulically efficient section");
    println!("{efficient}\n");

    // Second pass: the wider section an operating canal usually gets, set by a
    // bed width to depth ratio read from KP-03 against the discharge.
    let proportioned = design_channel(&ChannelDesignInput {
        design_discharge,
        bed_slope,
        side_slope,
        roughness,
        proportion: SectionProportion::BedWidthDepthRatio(2.0),
        velocity_limits,
        freeboard_rule: &freeboard,
        max_froude: Some(0.7),
        min_freeboard: None,
        solver: NormalDepthOptions::default(),
    })?;

    println!("Option B, bed width to depth ratio b/h = 2.0");
    println!("{proportioned}\n");

    // Third pass: the bed width rounded to a constructable 1.80 m, with the
    // depth solved from the discharge.
    let rounded = design_channel(&ChannelDesignInput {
        design_discharge,
        bed_slope,
        side_slope,
        roughness,
        proportion: SectionProportion::FixedBedWidth(Length::new(1.80)?),
        velocity_limits,
        freeboard_rule: &freeboard,
        max_froude: Some(0.7),
        min_freeboard: None,
        solver: NormalDepthOptions::default(),
    })?;

    println!("Option C, bed width rounded to 1.80 m, depth solved");
    println!("{rounded}\n");

    println!("Comparison");
    println!(
        "  {:<12} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "option", "b (m)", "h (m)", "P (m)", "V (m/s)", "checks"
    );
    for (name, design) in [
        ("A efficient", &efficient),
        ("B b/h = 2.0", &proportioned),
        ("C b = 1.80", &rounded),
    ] {
        println!(
            "  {:<12} {:>8.3} {:>8.3} {:>8.3} {:>8.3} {:>10}",
            name,
            design.bed_width.value(),
            design.flow_depth.value(),
            design.geometry().wetted_perimeter.value(),
            design.flow.velocity.value(),
            if design.all_checks_pass() {
                "pass"
            } else {
                "FAIL"
            }
        );
    }
    println!(
        "\nOption A has the smallest wetted perimeter, which is what makes it the\n\
         hydraulically efficient shape, but it is the deepest and narrowest of the\n\
         three. Canal choice weighs excavation against bank stability, freeboard\n\
         and maintenance access, and is not settled by hydraulics alone."
    );
    println!(
        "\nEvery result above is a calculation aid. It is not a substitute for a\n\
         licensed engineer's judgement and must be checked against the governing\n\
         Kriteria Perencanaan."
    );

    Ok(())
}
