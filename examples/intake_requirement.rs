//! Works an irrigation water requirement from the field up to the intake, then
//! checks the tertiary block layout and a short supply.
//!
//! Run with:
//!
//! ```text
//! cargo run -p kp-irigasi --example intake_requirement
//! ```
//!
//! The figures used here are illustrative. Crop coefficients, percolation,
//! rainfall and efficiencies are project data. This example is a calculation
//! aid, not a design.

use kp_irigasi::tertiary::{assess_tertiary_block, blocks_in_command_area, TertiaryCriteria};
use kp_irigasi::units::{Days, DepthRate, Discharge, Hectares, Length, Ratio, WaterLayer};
use kp_irigasi::water::{
    assess_short_supply, crop_water_requirement, diversion_requirement, effective_rainfall,
    land_preparation_requirement, net_field_requirement, open_water_evaporation, EfficiencyChain,
    RotationCriteria, DEFAULT_EO_FACTOR, DEFAULT_RICE_RAINFALL_COEFFICIENT,
};
use kp_irigasi::KpError;

fn main() -> Result<(), KpError> {
    let area = Hectares::new(1250.0)?;
    let period = Days::new(15.0)?; // half month, the KP-01 planning step
    let reference_et = DepthRate::new(4.60)?; // Eto from the climate station
    let percolation = DepthRate::new(2.00)?; // soil property, project data

    println!("Daerah irigasi Contoh, secondary block SS.1");
    println!(
        "Command area {} ha, planning period {} days\n",
        area.value(),
        period.value()
    );

    // --- Growing period, rice at mid season -------------------------------
    let crop_coefficient = Ratio::new(1.10)?; // kc for rice, mid season
    let etc = crop_water_requirement(crop_coefficient, reference_et)?;
    let water_layer_replacement = WaterLayer::new(50.0)?.spread_over(Days::new(30.0)?)?;
    let rainfall = effective_rainfall(
        WaterLayer::new(90.0)?,
        period,
        DEFAULT_RICE_RAINFALL_COEFFICIENT,
    )?;
    let field = net_field_requirement(etc, percolation, water_layer_replacement, rainfall)?;

    println!("Net field requirement, growing period");
    println!(
        "  Eto                          {:>8.3} mm/day",
        reference_et.value()
    );
    println!(
        "  kc                           {:>8.3}",
        crop_coefficient.value()
    );
    println!("  Etc = kc Eto                 {:>8.3} mm/day", etc.value());
    println!(
        "  percolation P                {:>8.3} mm/day",
        percolation.value()
    );
    println!(
        "  water layer replacement WLR  {:>8.3} mm/day  (50 mm over 30 days)",
        water_layer_replacement.value()
    );
    println!(
        "  effective rainfall Re        {:>8.3} mm/day  (0.7 × R80 90 mm / 15 days)",
        rainfall.value()
    );
    println!(
        "  balance Etc + P + WLR - Re   {:>8.3} mm/day",
        field.balance
    );
    println!(
        "  net field requirement NFR    {:>8.3} mm/day\n",
        field.net_field_requirement.value()
    );

    // --- Land preparation, the usual governing period ---------------------
    let open_water = open_water_evaporation(DepthRate::new(5.00)?, DEFAULT_EO_FACTOR)?;
    let land_prep = land_preparation_requirement(
        open_water,
        percolation,
        Days::new(30.0)?,
        WaterLayer::new(250.0)?,
    )?;
    let land_prep_field = net_field_requirement(
        land_prep.requirement,
        DepthRate::new(0.0)?, // percolation is already inside M, do not count it twice
        DepthRate::new(0.0)?,
        rainfall,
    )?;

    println!("Net field requirement, land preparation (van de Goor and Zijlstra)");
    println!(
        "  Eo = 1.1 Eto                 {:>8.3} mm/day",
        open_water.value()
    );
    println!(
        "  M = Eo + P                   {:>8.3} mm/day",
        land_prep.saturated_field_loss.value()
    );
    println!(
        "  T = 30 days, S = 250 mm, k = M T / S  {:>6.4}",
        land_prep.k
    );
    println!(
        "  IR = M e^k / (e^k - 1)       {:>8.3} mm/day",
        land_prep.requirement.value()
    );
    println!(
        "  NFR during land preparation  {:>8.3} mm/day\n",
        land_prep_field.net_field_requirement.value()
    );

    // --- Up the efficiency chain to the intake ----------------------------
    let efficiency = EfficiencyChain::new(0.80, 0.90, 0.90)?;
    let governing =
        if land_prep_field.net_field_requirement.value() > field.net_field_requirement.value() {
            land_prep_field.net_field_requirement
        } else {
            field.net_field_requirement
        };
    let requirement = diversion_requirement(governing, efficiency, area)?;

    println!("Governing NFR is the larger of the two periods: {governing:.3}");
    println!("{requirement}\n");

    // --- Tertiary block layout --------------------------------------------
    let criteria = TertiaryCriteria::default();
    let blocks = blocks_in_command_area(area, criteria.typical_area)?;
    println!(
        "Tertiary blocks (petak tersier) at the typical {} ha: {} blocks\n",
        criteria.typical_area.value(),
        blocks
    );

    let block = assess_tertiary_block(
        Hectares::new(96.0)?,
        Length::new(1200.0)?,
        Length::new(420.0)?,
        criteria,
    )?;
    println!("{block}\n");

    // --- Short supply -----------------------------------------------------
    let available = Discharge::from_litres_per_second(1350.0)?;
    let short = assess_short_supply(
        available,
        requirement.intake_discharge,
        area,
        RotationCriteria::default(),
    )?;
    println!("Short supply assessment");
    println!(
        "  available at the intake      {:>8.1} l/s",
        short.available.as_litres_per_second()
    );
    println!(
        "  required at the intake       {:>8.1} l/s",
        short.required.as_litres_per_second()
    );
    println!("  k factor                     {:>8.3}", short.k_factor);
    println!(
        "  relative area at full duty   {:>8.1} ha of {:.1} ha planned",
        short.relative_area.value(),
        short.planned_area.value()
    );
    println!("  indicated operation          {}", short.mode);
    println!(
        "\n  The rotation thresholds are this crate's documented assumption, not a\n\
         \x20 quotation from KP-01. Set them to the scheme's own operating rule."
    );

    println!(
        "\nEvery result above is a calculation aid. It is not a substitute for a\n\
         licensed engineer's judgement and must be checked against the governing\n\
         Kriteria Perencanaan and the scheme water balance."
    );

    Ok(())
}
