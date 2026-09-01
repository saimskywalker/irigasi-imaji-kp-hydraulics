//! Worked examples for irrigation water requirement and tertiary block sizing.
//!
//! Every expected value is hand calculated in the comment above the assertion.
//! Depth rates are mm/day, areas hectares, discharges m³/s.

use kp_irigasi::error::KpError;
use kp_irigasi::tertiary::{assess_tertiary_block, blocks_in_command_area, TertiaryCriteria};
use kp_irigasi::units::{Days, DepthRate, Discharge, Hectares, Length, Ratio, WaterLayer};
use kp_irigasi::water::{
    assess_short_supply, crop_water_requirement, diversion_requirement, effective_rainfall,
    land_preparation_requirement, net_field_requirement, open_water_evaporation, EfficiencyChain,
    RotationCriteria, SupplyMode, DEFAULT_EO_FACTOR, DEFAULT_RICE_RAINFALL_COEFFICIENT,
};

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

#[test]
fn crop_water_requirement_matches_hand_calculation() {
    // kc = 1.10, Eto = 4.60 mm/day: Etc = 5.06 mm/day
    let etc =
        crop_water_requirement(Ratio::new(1.10).unwrap(), DepthRate::new(4.60).unwrap()).unwrap();
    assert!(close(etc.value(), 5.06, 1e-12));
}

#[test]
fn effective_rainfall_for_rice_matches_hand_calculation() {
    // R80 = 90 mm over 15 days, coefficient 0.7: Re = 0.7 × 90 / 15 = 4.2 mm/day
    let re = effective_rainfall(
        WaterLayer::new(90.0).unwrap(),
        Days::new(15.0).unwrap(),
        DEFAULT_RICE_RAINFALL_COEFFICIENT,
    )
    .unwrap();
    assert!(close(re.value(), 4.2, 1e-12));
}

#[test]
fn effective_rainfall_rejects_a_coefficient_above_one() {
    let result = effective_rainfall(
        WaterLayer::new(90.0).unwrap(),
        Days::new(15.0).unwrap(),
        1.4,
    );
    assert!(matches!(
        result,
        Err(KpError::OutOfRange {
            parameter: "rainfall_coefficient",
            ..
        })
    ));
}

#[test]
fn net_field_requirement_matches_hand_calculation() {
    // Etc = 5.06, P = 2.00, WLR = 1.67, Re = 4.20 mm/day
    // NFR = 5.06 + 2.00 + 1.67 - 4.20 = 4.53 mm/day
    let nfr = net_field_requirement(
        DepthRate::new(5.06).unwrap(),
        DepthRate::new(2.00).unwrap(),
        DepthRate::new(1.67).unwrap(),
        DepthRate::new(4.20).unwrap(),
    )
    .unwrap();
    assert!(close(nfr.net_field_requirement.value(), 4.53, 1e-12));
    assert!(close(nfr.balance, 4.53, 1e-12));
    assert!(!nfr.rainfall_surplus);
}

#[test]
fn a_rainfall_surplus_floors_the_requirement_at_zero_and_says_so() {
    // Etc = 3.00, P = 1.50, WLR = 0, Re = 7.20 mm/day
    // Balance = 3.00 + 1.50 + 0 - 7.20 = -2.70 mm/day, so no irrigation is
    // needed, but the raw balance is still reported.
    let nfr = net_field_requirement(
        DepthRate::new(3.00).unwrap(),
        DepthRate::new(1.50).unwrap(),
        DepthRate::new(0.0).unwrap(),
        DepthRate::new(7.20).unwrap(),
    )
    .unwrap();
    assert!(close(nfr.balance, -2.70, 1e-12));
    assert!(close(nfr.net_field_requirement.value(), 0.0, 1e-12));
    assert!(nfr.rainfall_surplus);
}

#[test]
fn water_layer_replacement_spreads_over_its_period() {
    // 50 mm applied over 30 days is 1.6666667 mm/day.
    let wlr = WaterLayer::new(50.0)
        .unwrap()
        .spread_over(Days::new(30.0).unwrap())
        .unwrap();
    assert!(close(wlr.value(), 1.666_666_7, 1e-7));
}

#[test]
fn land_preparation_requirement_matches_hand_calculation() {
    // Eto = 5.0 mm/day, Eo = 1.1 × 5.0 = 5.5 mm/day, P = 2.0 mm/day
    // M = 7.5 mm/day, T = 30 days, S = 250 mm
    // k = 7.5 × 30 / 250 = 0.9
    // e^0.9 = 2.4596031
    // IR = 7.5 × 2.4596031 / 1.4596031 = 12.6383831 mm/day
    let eo = open_water_evaporation(DepthRate::new(5.0).unwrap(), DEFAULT_EO_FACTOR).unwrap();
    assert!(close(eo.value(), 5.5, 1e-12));

    let lp = land_preparation_requirement(
        eo,
        DepthRate::new(2.0).unwrap(),
        Days::new(30.0).unwrap(),
        WaterLayer::new(250.0).unwrap(),
    )
    .unwrap();
    assert!(close(lp.saturated_field_loss.value(), 7.5, 1e-12));
    assert!(close(lp.k, 0.9, 1e-12));
    assert!(close(lp.requirement.value(), 12.638_383_1, 1e-6));
}

#[test]
fn land_preparation_requirement_always_exceeds_the_saturated_field_loss() {
    // IR = M e^k / (e^k - 1) > M for every positive k, because the field has to
    // be filled as well as kept wet.
    for (t, s) in [(30.0, 250.0), (45.0, 300.0), (20.0, 200.0)] {
        let lp = land_preparation_requirement(
            DepthRate::new(5.5).unwrap(),
            DepthRate::new(2.0).unwrap(),
            Days::new(t).unwrap(),
            WaterLayer::new(s).unwrap(),
        )
        .unwrap();
        assert!(lp.requirement.value() > lp.saturated_field_loss.value());
    }
}

#[test]
fn land_preparation_rejects_a_zero_period() {
    assert!(matches!(
        Days::new(0.0),
        Err(KpError::NotPositive {
            parameter: "Days",
            ..
        })
    ));
}

#[test]
fn the_efficiency_chain_multiplies() {
    // 0.80 × 0.90 × 0.90 = 0.648
    let chain = EfficiencyChain::new(0.80, 0.90, 0.90).unwrap();
    assert!(close(chain.overall(), 0.648, 1e-12));
    assert!(close(EfficiencyChain::default().overall(), 0.648, 1e-12));
}

#[test]
fn the_efficiency_chain_rejects_impossible_efficiencies() {
    assert!(matches!(
        EfficiencyChain::new(0.0, 0.9, 0.9),
        Err(KpError::NotPositive { .. })
    ));
    assert!(matches!(
        EfficiencyChain::new(0.8, 1.2, 0.9),
        Err(KpError::OutOfRange { .. })
    ));
}

#[test]
fn diversion_requirement_matches_hand_calculation() {
    // NFR = 1.30 mm/day, efficiencies 0.80 / 0.90 / 0.90, area 1 250 ha
    // At the tertiary offtake: 1.30 / 0.80         = 1.6250000 mm/day
    // At the secondary head:   1.6250000 / 0.90    = 1.8055556 mm/day
    // At the intake:           1.8055556 / 0.90    = 2.0061728 mm/day
    // Unit discharge:          2.0061728 / 8.64    = 0.2321959 l/s/ha
    // Intake discharge:        0.2321959 × 1250    = 290.2449 l/s
    let dr = diversion_requirement(
        DepthRate::new(1.30).unwrap(),
        EfficiencyChain::new(0.80, 0.90, 0.90).unwrap(),
        Hectares::new(1250.0).unwrap(),
    )
    .unwrap();
    assert!(close(dr.at_tertiary_offtake.value(), 1.625, 1e-12));
    assert!(close(dr.at_secondary_head.value(), 1.805_555_6, 1e-7));
    assert!(close(dr.at_intake.value(), 2.006_172_8, 1e-7));
    assert!(close(dr.unit_discharge_lps_per_ha, 0.232_195_9, 1e-7));
    assert!(close(
        dr.intake_discharge.as_litres_per_second(),
        290.244_913,
        1e-4
    ));
    assert!(close(dr.overall_efficiency, 0.648, 1e-12));
}

#[test]
fn dividing_by_the_overall_efficiency_gives_the_same_answer_as_the_chain() {
    // A cross check on the chain: NFR / 0.648 must equal the intake figure.
    let dr = diversion_requirement(
        DepthRate::new(1.30).unwrap(),
        EfficiencyChain::default(),
        Hectares::new(1250.0).unwrap(),
    )
    .unwrap();
    assert!(close(dr.at_intake.value(), 1.30 / 0.648, 1e-9));
}

#[test]
fn the_mm_per_day_to_discharge_conversion_is_the_published_one() {
    // 1 mm/day over 1 ha = 10 m³/day = 10 000 l / 86 400 s = 0.1157407 l/s,
    // that is 1 / 8.64.
    let unit = DepthRate::new(1.0).unwrap();
    assert!(close(
        unit.as_litres_per_second_per_hectare(),
        0.115_740_7,
        1e-7
    ));
    let q = unit.over_area(Hectares::new(1.0).unwrap()).unwrap();
    assert!(close(q.as_litres_per_second(), 0.115_740_7, 1e-7));
}

#[test]
fn a_discharge_converts_back_to_a_depth_rate_over_the_same_area() {
    let rate = DepthRate::new(2.006_172_8).unwrap();
    let area = Hectares::new(1250.0).unwrap();
    let back = rate
        .over_area(area)
        .unwrap()
        .as_depth_rate_over(area)
        .unwrap();
    assert!(close(back.value(), rate.value(), 1e-9));
}

#[test]
fn short_supply_gives_the_k_factor_and_the_relative_area() {
    // Available 0.42 m³/s against 0.60 m³/s required: k = 0.70
    // Relative area = 0.70 × 800 = 560 ha
    let s = assess_short_supply(
        Discharge::new(0.42).unwrap(),
        Discharge::new(0.60).unwrap(),
        Hectares::new(800.0).unwrap(),
        RotationCriteria::default(),
    )
    .unwrap();
    assert!(close(s.k_factor, 0.70, 1e-12));
    assert!(close(s.relative_area.value(), 560.0, 1e-9));
    assert_eq!(s.mode, SupplyMode::Rotation { groups: 2 });
}

#[test]
fn the_k_factor_is_capped_at_one_when_supply_exceeds_demand() {
    let s = assess_short_supply(
        Discharge::new(1.20).unwrap(),
        Discharge::new(0.60).unwrap(),
        Hectares::new(800.0).unwrap(),
        RotationCriteria::default(),
    )
    .unwrap();
    assert!(close(s.k_factor, 1.0, 1e-12));
    assert!(close(s.relative_area.value(), 800.0, 1e-9));
    assert_eq!(s.mode, SupplyMode::Continuous);
}

#[test]
fn the_supply_mode_follows_the_rotation_thresholds() {
    let modes = |k: f64| {
        assess_short_supply(
            Discharge::new(k).unwrap(),
            Discharge::new(1.0).unwrap(),
            Hectares::new(500.0).unwrap(),
            RotationCriteria::default(),
        )
        .unwrap()
        .mode
    };
    assert_eq!(modes(0.90), SupplyMode::Continuous);
    assert_eq!(modes(0.80), SupplyMode::Continuous);
    assert_eq!(modes(0.65), SupplyMode::Rotation { groups: 2 });
    assert_eq!(modes(0.50), SupplyMode::Rotation { groups: 3 });
    assert_eq!(modes(0.30), SupplyMode::Insufficient);
}

#[test]
fn short_supply_rejects_a_zero_requirement() {
    let result = assess_short_supply(
        Discharge::new(0.42).unwrap(),
        Discharge::new(0.0).unwrap(),
        Hectares::new(800.0).unwrap(),
        RotationCriteria::default(),
    );
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "required_discharge",
            ..
        })
    ));
}

#[test]
fn tertiary_block_assessment_matches_hand_calculation() {
    // 96 ha block, tertiary channel 1 200 m, longest quaternary channel 420 m.
    // Quaternary blocks = ceil(96 / 15) = 7
    // Mean quaternary area = 96 / 7 = 13.7142857 ha
    let block = assess_tertiary_block(
        Hectares::new(96.0).unwrap(),
        Length::new(1200.0).unwrap(),
        Length::new(420.0).unwrap(),
        TertiaryCriteria::default(),
    )
    .unwrap();
    assert_eq!(block.quaternary_blocks, 7);
    assert!(close(
        block.mean_quaternary_area.value(),
        13.714_285_7,
        1e-7
    ));
    assert!(block.all_checks_pass());
    assert_eq!(block.checks.len(), 4);
}

#[test]
fn an_oversized_tertiary_block_fails_its_checks() {
    // 200 ha against a 150 ha maximum, and a 2 000 m tertiary channel against
    // a 1 500 m maximum: two failures, and the block is still returned.
    let block = assess_tertiary_block(
        Hectares::new(200.0).unwrap(),
        Length::new(2000.0).unwrap(),
        Length::new(420.0).unwrap(),
        TertiaryCriteria::default(),
    )
    .unwrap();
    assert!(!block.all_checks_pass());
    let failed: Vec<_> = block
        .checks
        .iter()
        .filter(|c| c.status == kp_irigasi::check::CheckStatus::Fail)
        .map(|c| c.name)
        .collect();
    assert!(failed.contains(&"tertiary block area"));
    assert!(failed.contains(&"tertiary channel length"));
}

#[test]
fn the_number_of_blocks_a_secondary_canal_serves_rounds_up() {
    // 640 ha in 50 ha blocks: ceil(12.8) = 13
    assert_eq!(
        blocks_in_command_area(Hectares::new(640.0).unwrap(), Hectares::new(50.0).unwrap())
            .unwrap(),
        13
    );
    // An exact division does not round up.
    assert_eq!(
        blocks_in_command_area(Hectares::new(600.0).unwrap(), Hectares::new(50.0).unwrap())
            .unwrap(),
        12
    );
    // A command area smaller than one block still needs one block.
    assert_eq!(
        blocks_in_command_area(Hectares::new(12.0).unwrap(), Hectares::new(50.0).unwrap()).unwrap(),
        1
    );
}

#[test]
fn block_counting_rejects_a_zero_block_area() {
    let result = blocks_in_command_area(Hectares::new(640.0).unwrap(), Hectares::new(0.0).unwrap());
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "block_area",
            ..
        })
    ));
}
