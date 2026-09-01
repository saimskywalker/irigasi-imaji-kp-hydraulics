//! Worked examples for hydraulic structures, drops, culverts and siphons.
//!
//! Every expected value is hand calculated in the comment above the assertion,
//! using g = 9.81 m/s². Units: m, m², m/s, m³/s.

use kp_irigasi::conduit::{
    conduit_head_loss, size_circular_barrel, Barrel, BarrelSizingOptions, ConduitInput,
    LossCoefficients,
};
use kp_irigasi::error::KpError;
use kp_irigasi::structures::{
    broad_crested_weir_discharge, cipoletti_weir_discharge, free_orifice_discharge, hydraulic_jump,
    rectangular_weir_discharge, rehbock_coefficient, sharp_crested_weir_discharge,
    sluice_gate_discharge, straight_drop, submerged_orifice_discharge, v_notch_weir_discharge,
    DEFAULT_JUMP_LENGTH_FACTOR,
};
use kp_irigasi::uniform_flow::VelocityLimits;
use kp_irigasi::units::{Area, Discharge, Length, ManningN, Ratio};

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

#[test]
fn sharp_crested_weir_matches_hand_calculation() {
    // Cd = 0.62, b = 1.50 m, H = 0.30 m
    // sqrt(2g) = 4.4294469, H^1.5 = 0.1643168
    // Q = (2/3) × 0.62 × 1.50 × 4.4294469 × 0.1643168 = 0.4512561 m³/s
    let q = sharp_crested_weir_discharge(
        Ratio::new(0.62).unwrap(),
        Length::new(1.50).unwrap(),
        Length::new(0.30).unwrap(),
    )
    .unwrap();
    assert!(close(q.value(), 0.451_256_1, 1e-7));
}

#[test]
fn weir_discharge_follows_the_three_halves_power_of_head() {
    // Doubling the head multiplies the discharge by 2^1.5 = 2.8284271.
    let at = |h: f64| {
        sharp_crested_weir_discharge(
            Ratio::new(0.62).unwrap(),
            Length::new(1.50).unwrap(),
            Length::new(h).unwrap(),
        )
        .unwrap()
        .value()
    };
    assert!(close(at(0.60) / at(0.30), 2.828_427_1, 1e-7));
}

#[test]
fn rehbock_coefficient_matches_hand_calculation() {
    // H = 0.30 m, P = 0.60 m: Cd = 0.602 + 0.083 × 0.5 = 0.6435
    let cd = rehbock_coefficient(Length::new(0.30).unwrap(), Length::new(0.60).unwrap()).unwrap();
    assert!(close(cd.value(), 0.6435, 1e-12));
}

#[test]
fn broad_crested_weir_matches_hand_calculation() {
    // Q = Cd Cv (2/3) sqrt(2g/3) b h^1.5, with (2/3) sqrt(2 × 9.81 / 3)
    //   = 0.6666667 × 2.5573424 = 1.7048949
    // Cd = 0.93, Cv = 1.02, b = 2.00 m, h = 0.45 m, h^1.5 = 0.3018692
    // Q = 1.7048949 × 0.9486 × 2.00 × 0.3018692 = 0.9764039 m³/s
    let q = broad_crested_weir_discharge(
        Ratio::new(0.93).unwrap(),
        Ratio::new(1.02).unwrap(),
        Length::new(2.00).unwrap(),
        Length::new(0.45).unwrap(),
    )
    .unwrap();
    assert!(close(q.value(), 0.976_403_9, 1e-7));
}

#[test]
fn free_and_submerged_orifice_match_hand_calculations() {
    // Free: Cd = 0.62, A = 0.20 m², H = 0.80 m
    //   sqrt(2 × 9.81 × 0.80) = 3.9618177
    //   Q = 0.124 × 3.9618177 = 0.4912654 m³/s
    let free = free_orifice_discharge(
        Ratio::new(0.62).unwrap(),
        Area::new(0.20).unwrap(),
        Length::new(0.80).unwrap(),
    )
    .unwrap();
    assert!(close(free.value(), 0.491_265_4, 1e-7));

    // Submerged: Cd = 0.80, A = 0.30 m², dh = 0.12 m
    //   sqrt(2 × 9.81 × 0.12) = 1.5344054
    //   Q = 0.24 × 1.5344054 = 0.3682573 m³/s
    let submerged = submerged_orifice_discharge(
        Ratio::new(0.80).unwrap(),
        Area::new(0.30).unwrap(),
        Length::new(0.12).unwrap(),
    )
    .unwrap();
    assert!(close(submerged.value(), 0.368_257_3, 1e-7));
}

#[test]
fn an_orifice_coefficient_above_one_is_rejected() {
    let result = free_orifice_discharge(
        Ratio::new(1.4).unwrap(),
        Area::new(0.20).unwrap(),
        Length::new(0.80).unwrap(),
    );
    assert!(matches!(
        result,
        Err(KpError::OutOfRange {
            parameter: "discharge_coefficient",
            ..
        })
    ));
}

#[test]
fn sluice_gate_matches_hand_calculation_and_scales_with_submergence() {
    // mu = 0.62, K = 1.0, a = 0.25 m, b = 1.20 m, z = 0.50 m
    // sqrt(2 × 9.81 × 0.50) = 3.1320920
    // Q = 0.62 × 0.25 × 1.20 × 3.1320920 = 0.5825691 m³/s
    let free = sluice_gate_discharge(
        Ratio::new(0.62).unwrap(),
        Ratio::new(1.0).unwrap(),
        Length::new(0.25).unwrap(),
        Length::new(1.20).unwrap(),
        Length::new(0.50).unwrap(),
    )
    .unwrap();
    assert!(close(free.value(), 0.582_569_1, 1e-7));

    // A submergence factor of 0.85 reduces the discharge in proportion.
    let drowned = sluice_gate_discharge(
        Ratio::new(0.62).unwrap(),
        Ratio::new(0.85).unwrap(),
        Length::new(0.25).unwrap(),
        Length::new(1.20).unwrap(),
        Length::new(0.50).unwrap(),
    )
    .unwrap();
    assert!(close(drowned.value(), 0.85 * free.value(), 1e-12));
}

#[test]
fn cipoletti_weir_matches_hand_calculation() {
    // b = 0.50 m, H = 0.20 m, H^1.5 = 0.0894427
    // Q = 1.859 × 0.50 × 0.0894427 = 0.0831370 m³/s = 83.137 l/s
    let q =
        cipoletti_weir_discharge(Length::new(0.50).unwrap(), Length::new(0.20).unwrap()).unwrap();
    assert!(close(q.as_litres_per_second(), 83.1370, 1e-3));
}

#[test]
fn rectangular_measuring_weir_accounts_for_end_contractions() {
    // Suppressed, b = 0.60 m, H = 0.15 m: Q = 1.84 × 0.60 × 0.0580948
    //   = 0.0641366 m³/s = 64.137 l/s
    let suppressed =
        rectangular_weir_discharge(Length::new(0.60).unwrap(), Length::new(0.15).unwrap(), 0)
            .unwrap();
    assert!(close(suppressed.as_litres_per_second(), 64.1366, 1e-3));

    // Two end contractions: b_eff = 0.60 - 0.1 × 2 × 0.15 = 0.57 m
    //   Q = 1.84 × 0.57 × 0.0580948 = 0.0609298 m³/s = 60.930 l/s
    let contracted =
        rectangular_weir_discharge(Length::new(0.60).unwrap(), Length::new(0.15).unwrap(), 2)
            .unwrap();
    assert!(close(contracted.as_litres_per_second(), 60.9298, 1e-3));
    assert!(contracted.value() < suppressed.value());
}

#[test]
fn a_head_that_swallows_the_crest_width_is_rejected() {
    // b = 0.20 m with H = 1.20 m gives b_eff = 0.20 - 0.24 = -0.04 m, which is
    // not a weir any more.
    let result =
        rectangular_weir_discharge(Length::new(0.20).unwrap(), Length::new(1.20).unwrap(), 2);
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "effective_crest_width",
            ..
        })
    ));
}

#[test]
fn v_notch_weir_matches_hand_calculation() {
    // 90 degree notch, Cd = 0.60, H = 0.20 m
    // (8/15) × 0.60 × 4.4294469 × tan(45°) = 1.4174231
    // H^2.5 = 0.0178885
    // Q = 1.4174231 × 0.0178885 = 0.0253556 m³/s = 25.356 l/s
    let q = v_notch_weir_discharge(Ratio::new(0.60).unwrap(), 90.0, Length::new(0.20).unwrap())
        .unwrap();
    assert!(close(q.as_litres_per_second(), 25.3556, 1e-3));
}

#[test]
fn hydraulic_jump_matches_hand_calculation() {
    // y1 = 0.20 m, V1 = 4.00 m/s
    // Fr1 = 4.00 / sqrt(9.81 × 0.20) = 4.00 / 1.4007141 = 2.8556862
    // y2 = 0.5 × 0.20 × (sqrt(1 + 8 × 8.1549439) - 1)
    //    = 0.10 × (8.1387684 - 1) = 0.7138768 m
    // V2 = 4.00 × 0.20 / 0.7138768 = 1.1206415 m/s
    // dE = (0.5138768)³ / (4 × 0.20 × 0.7138768) = 0.2376095 m
    let jump = hydraulic_jump(
        Length::new(0.20).unwrap(),
        kp_irigasi::units::Velocity::new(4.00).unwrap(),
    )
    .unwrap();
    assert!(close(jump.initial_froude, 2.855_686_2, 1e-7));
    assert!(close(jump.sequent_depth.value(), 0.713_876_8, 1e-7));
    assert!(close(jump.sequent_velocity.value(), 1.120_641_5, 1e-7));
    assert!(close(jump.energy_loss.value(), 0.237_609_5, 1e-7));

    // The energy loss must equal the drop in specific energy.
    assert!(close(
        jump.initial_energy.value() - jump.sequent_energy.value(),
        jump.energy_loss.value(),
        1e-9
    ));
}

#[test]
fn no_jump_forms_in_subcritical_flow() {
    // y1 = 1.00 m, V1 = 0.50 m/s gives Fr1 = 0.16, so there is nothing to
    // solve and the function says so rather than returning a nonsense depth.
    let result = hydraulic_jump(
        Length::new(1.00).unwrap(),
        kp_irigasi::units::Velocity::new(0.50).unwrap(),
    );
    assert!(matches!(
        result,
        Err(KpError::OutOfRange {
            parameter: "initial_froude",
            ..
        })
    ));
}

#[test]
fn straight_drop_matches_hand_calculation() {
    // q = 0.60 m³/s per m, dz = 1.00 m
    // D = 0.36 / (9.81 × 1) = 0.03669725
    // yc = (0.36 / 9.81)^(1/3) = 0.3323108 m
    // D^0.27 = 0.4096859, D^0.425 = 0.2454535, D^0.22 = 0.4833029
    // Ld = 4.30 × 0.4096859 = 1.7616495 m
    // yp = 1.00 × 0.4833029 = 0.4833029 m
    // y1 = 0.54 × 0.2454535 = 0.1325449 m
    // y2 = 1.66 × 0.4096859 = 0.6800786 m
    // jump length = 6 × (0.6800786 - 0.1325449) = 3.2852025 m
    // basin length = 1.7616495 + 3.2852025 = 5.0468519 m
    // dE = (0.5475337)³ / (4 × 0.1325449 × 0.6800786) = 0.4552506 m
    let drop = straight_drop(0.60, Length::new(1.00).unwrap(), DEFAULT_JUMP_LENGTH_FACTOR).unwrap();
    assert!(close(drop.drop_number, 0.036_697_248, 1e-9));
    assert!(close(drop.critical_depth.value(), 0.332_310_8, 1e-7));
    assert!(close(drop.nappe_length.value(), 1.761_649_5, 1e-7));
    assert!(close(drop.pool_depth.value(), 0.483_302_9, 1e-7));
    assert!(close(drop.toe_depth.value(), 0.132_544_9, 1e-7));
    assert!(close(drop.sequent_depth.value(), 0.680_078_6, 1e-7));
    assert!(close(drop.jump_length.value(), 3.285_202_5, 1e-7));
    assert!(close(drop.basin_length.value(), 5.046_851_9, 1e-7));
    assert!(close(drop.energy_loss.value(), 0.455_250_6, 1e-7));
}

#[test]
fn the_toe_of_a_drop_is_supercritical_and_the_sequent_depth_is_deeper() {
    let drop = straight_drop(0.60, Length::new(1.00).unwrap(), DEFAULT_JUMP_LENGTH_FACTOR).unwrap();
    assert!(drop.toe_depth.value() < drop.critical_depth.value());
    assert!(drop.sequent_depth.value() > drop.critical_depth.value());
}

#[test]
fn a_zero_drop_height_is_rejected() {
    let result = straight_drop(0.60, Length::new(0.0).unwrap(), DEFAULT_JUMP_LENGTH_FACTOR);
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "drop_height",
            ..
        })
    ));
}

#[test]
fn circular_culvert_head_loss_matches_hand_calculation() {
    // D = 0.80 m, Q = 0.60 m³/s, L = 24 m, n = 0.014, k_in = 0.5, k_out = 1.0
    // A = pi × 0.64 / 4 = 0.5026548 m²
    // V = 0.60 / 0.5026548 = 1.1936621 m/s
    // hv = 1.1936621² / 19.62 = 0.0726213 m
    // R = D / 4 = 0.20 m, R^(2/3) = 0.3419952
    // hf = 24 × (0.014 × 1.1936621 / 0.3419952)² = 24 × 0.0023877 = 0.0573047 m
    // H = 1.5 × 0.0726213 + 0.0573047 = 0.1662366 m
    let loss = conduit_head_loss(&ConduitInput {
        barrel: Barrel::Circular {
            diameter: Length::new(0.80).unwrap(),
        },
        discharge: Discharge::new(0.60).unwrap(),
        length: Length::new(24.0).unwrap(),
        roughness: ManningN::new(0.014).unwrap(),
        coefficients: LossCoefficients::new(0.5, 1.0, 0.0).unwrap(),
        velocity_limits: VelocityLimits::from_range(0.60, 3.0).unwrap(),
    })
    .unwrap();

    assert!(close(loss.flow_area.value(), 0.502_654_8, 1e-7));
    assert!(close(loss.velocity.value(), 1.193_662_1, 1e-7));
    assert!(close(loss.velocity_head.value(), 0.072_621_3, 1e-7));
    assert!(close(loss.hydraulic_radius.value(), 0.20, 1e-12));
    assert!(close(loss.entry_loss.value(), 0.036_310_6, 1e-7));
    assert!(close(loss.exit_loss.value(), 0.072_621_3, 1e-7));
    assert!(close(loss.friction_loss.value(), 0.057_304_7, 1e-7));
    assert!(close(loss.total_head_loss.value(), 0.166_236_6, 1e-7));
    assert!(loss.all_checks_pass());
}

#[test]
fn rectangular_barrel_head_loss_matches_hand_calculation() {
    // 1.00 m × 0.80 m box, Q = 1.00 m³/s, L = 15 m, n = 0.017
    // A = 0.80 m², P = 3.60 m, R = 0.2222222 m
    // V = 1.25 m/s, hv = 1.25² / 19.62 = 0.0796381 m
    // R^(2/3) = 0.3667195
    // hf = 15 × (0.017 × 1.25 / 0.3667195)² = 15 × 0.0033548 = 0.0503221 m
    // H = 1.5 × 0.0796381 + 0.0503221 = 0.1697793 m
    let loss = conduit_head_loss(&ConduitInput {
        barrel: Barrel::Rectangular {
            width: Length::new(1.00).unwrap(),
            height: Length::new(0.80).unwrap(),
        },
        discharge: Discharge::new(1.00).unwrap(),
        length: Length::new(15.0).unwrap(),
        roughness: ManningN::new(0.017).unwrap(),
        coefficients: LossCoefficients::new(0.5, 1.0, 0.0).unwrap(),
        velocity_limits: VelocityLimits::from_range(0.60, 3.0).unwrap(),
    })
    .unwrap();
    assert!(close(loss.velocity.value(), 1.25, 1e-12));
    assert!(close(loss.hydraulic_radius.value(), 0.222_222_2, 1e-7));
    assert!(close(loss.friction_loss.value(), 0.050_322_1, 1e-7));
    assert!(close(loss.total_head_loss.value(), 0.169_779_3, 1e-7));
}

#[test]
fn bend_losses_add_to_the_total() {
    let base = |bends: f64| {
        conduit_head_loss(&ConduitInput {
            barrel: Barrel::Circular {
                diameter: Length::new(0.80).unwrap(),
            },
            discharge: Discharge::new(0.60).unwrap(),
            length: Length::new(24.0).unwrap(),
            roughness: ManningN::new(0.014).unwrap(),
            coefficients: LossCoefficients::new(0.5, 1.0, bends).unwrap(),
            velocity_limits: VelocityLimits::default(),
        })
        .unwrap()
    };
    let without = base(0.0);
    let with = base(0.40);
    // Two siphon elbows at 0.20 each add 0.40 × hv = 0.40 × 0.0726213
    //   = 0.0290485 m.
    assert!(close(with.bend_loss.value(), 0.029_048_5, 1e-7));
    assert!(close(
        with.total_head_loss.value() - without.total_head_loss.value(),
        0.029_048_5,
        1e-7
    ));
}

#[test]
fn barrel_sizing_finds_the_smallest_diameter_within_the_allowance() {
    // Q = 0.60 m³/s, L = 24 m, n = 0.014, k_in = 0.5, k_out = 1.0,
    // allowable head loss 0.20 m. Bisection on diameter converges on
    // D = 0.767568 m, at which the head loss is exactly the 0.20 m allowance.
    let sizing = size_circular_barrel(
        Discharge::new(0.60).unwrap(),
        Length::new(24.0).unwrap(),
        ManningN::new(0.014).unwrap(),
        LossCoefficients::new(0.5, 1.0, 0.0).unwrap(),
        Length::new(0.20).unwrap(),
        VelocityLimits::from_range(0.60, 3.0).unwrap(),
        BarrelSizingOptions::default(),
    )
    .unwrap();

    assert!(close(sizing.diameter.value(), 0.767_568, 1e-4));
    assert!(sizing.head_loss.total_head_loss.value() <= 0.20);
    assert!(sizing.iterations > 0);
    // One tolerance smaller and the allowance would be exceeded, which is what
    // makes this the smallest acceptable barrel.
    assert!(sizing.head_loss.total_head_loss.value() > 0.199);
}

#[test]
fn barrel_sizing_reports_no_bracket_when_the_allowance_cannot_be_met() {
    // A 200 m barrel carrying 3 m³/s cannot lose less than 1 mm of head at any
    // diameter up to 1.0 m.
    let result = size_circular_barrel(
        Discharge::new(3.0).unwrap(),
        Length::new(200.0).unwrap(),
        ManningN::new(0.014).unwrap(),
        LossCoefficients::new(0.5, 1.0, 0.0).unwrap(),
        Length::new(0.001).unwrap(),
        VelocityLimits::default(),
        BarrelSizingOptions {
            max_diameter: 1.0,
            ..BarrelSizingOptions::default()
        },
    );
    assert!(matches!(
        result,
        Err(KpError::NoBracket {
            routine: "size_circular_barrel",
            ..
        })
    ));
}

#[test]
fn barrel_sizing_reports_non_convergence_when_the_iterations_run_out() {
    let result = size_circular_barrel(
        Discharge::new(0.60).unwrap(),
        Length::new(24.0).unwrap(),
        ManningN::new(0.014).unwrap(),
        LossCoefficients::new(0.5, 1.0, 0.0).unwrap(),
        Length::new(0.20).unwrap(),
        VelocityLimits::default(),
        BarrelSizingOptions {
            max_iterations: 2,
            ..BarrelSizingOptions::default()
        },
    );
    assert!(matches!(
        result,
        Err(KpError::DidNotConverge {
            routine: "size_circular_barrel",
            iterations: 2,
            ..
        })
    ));
}

#[test]
fn a_barrel_sized_for_head_loss_can_be_too_slow_for_silt() {
    // A tight head allowance forces a large barrel, and a large barrel runs
    // slowly. Q = 0.10 m³/s through 20 m with only 5 mm of head to spend
    // needs D = 0.775 m, at which the velocity is 0.212 m/s, well under the
    // 1.0 m/s wanted to keep silt moving. The conflict is reported, not
    // resolved: the engineer has to find the head somewhere or accept
    // desilting.
    let sizing = size_circular_barrel(
        Discharge::new(0.10).unwrap(),
        Length::new(20.0).unwrap(),
        ManningN::new(0.014).unwrap(),
        LossCoefficients::new(0.5, 1.0, 0.0).unwrap(),
        Length::new(0.005).unwrap(),
        VelocityLimits::from_range(1.0, 3.0).unwrap(),
        BarrelSizingOptions::default(),
    )
    .unwrap();
    assert!(close(sizing.diameter.value(), 0.775_241, 1e-4));
    assert!(sizing.head_loss.velocity.value() < 1.0);
    assert!(!sizing.head_loss.all_checks_pass());
    assert!(sizing
        .head_loss
        .checks
        .iter()
        .any(|c| c.name == "minimum velocity" && c.status == kp_irigasi::check::CheckStatus::Fail));
}

#[test]
fn a_zero_barrel_dimension_is_rejected() {
    let barrel = Barrel::Circular {
        diameter: Length::new(0.0).unwrap(),
    };
    assert!(matches!(
        barrel.flow_area(),
        Err(KpError::NotPositive {
            parameter: "diameter",
            ..
        })
    ));
}
