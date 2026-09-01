//! Worked examples for canal section design and freeboard rules.
//!
//! Every expected value is hand calculated in the comment above the assertion.
//! Units follow the crate convention: m, m², m/s, m³/s.

use kp_irigasi::check::CheckStatus;
use kp_irigasi::design::{
    design_channel, efficient_trapezoid, ChannelDesignInput, SectionProportion,
};
use kp_irigasi::error::KpError;
use kp_irigasi::freeboard::{
    FixedFreeboard, FreeboardFn, FreeboardRule, FreeboardStep, FreeboardTable,
};
use kp_irigasi::uniform_flow::{FlowRegime, NormalDepthOptions, VelocityLimits};
use kp_irigasi::units::{Discharge, Length, ManningN, SideSlope, Slope};

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

fn base_input<'a>(freeboard: &'a FreeboardTable) -> ChannelDesignInput<'a> {
    ChannelDesignInput {
        design_discharge: Discharge::new(2.0).unwrap(),
        bed_slope: Slope::new(0.0005).unwrap(),
        side_slope: SideSlope::new(1.5).unwrap(),
        roughness: ManningN::new(0.025).unwrap(),
        proportion: SectionProportion::MostEfficient,
        velocity_limits: VelocityLimits::from_range(0.30, 0.70).unwrap(),
        freeboard_rule: freeboard,
        max_froude: Some(0.7),
        min_freeboard: None,
        solver: NormalDepthOptions::default(),
    }
}

#[test]
fn efficient_rectangle_is_twice_as_wide_as_it_is_deep() {
    // m = 0: b/h = 2(sqrt(1) - 0) = 2, A/h² = 2 × 1 - 0 = 2.
    let e = efficient_trapezoid(SideSlope::new(0.0).unwrap()).unwrap();
    assert!(close(e.bed_width_depth_ratio, 2.0, 1e-12));
    assert!(close(e.area_coefficient, 2.0, 1e-12));
}

#[test]
fn efficient_trapezoid_matches_hand_calculation() {
    // m = 1.5: sqrt(1 + 2.25) = 1.8027756
    // b/h = 2 × (1.8027756 - 1.5) = 0.6055513
    // A/h² = 2 × 1.8027756 - 1.5 = 2.1055513
    let e = efficient_trapezoid(SideSlope::new(1.5).unwrap()).unwrap();
    assert!(close(e.bed_width_depth_ratio, 0.605_551_3, 1e-7));
    assert!(close(e.area_coefficient, 2.105_551_3, 1e-7));
}

#[test]
fn the_efficient_section_has_a_hydraulic_radius_of_half_the_depth() {
    // The defining property of the efficient trapezoid: R = h/2 exactly, for
    // any side slope. Checked here through the design routine.
    let freeboard = FreeboardTable::default();
    let design = design_channel(&base_input(&freeboard)).unwrap();
    assert!(close(
        design.geometry().hydraulic_radius.value(),
        design.flow_depth.value() / 2.0,
        1e-9
    ));
}

#[test]
fn efficient_section_design_matches_hand_calculation() {
    // Q = 2.0 m³/s, S = 0.0005, m = 1.5, n = 0.025
    // Q n / sqrt(S) = 2.0 × 0.025 / 0.0223607 = 2.2360680
    // A R^(2/3) = ca h² (h/2)^(2/3) = (ca / 2^(2/3)) h^(8/3)
    //   ca = 2.1055513, 2^(2/3) = 1.5874011, so c = 1.3264142
    // h = (2.2360680 / 1.3264142)^(3/8) = 1.6857^0.375 = 1.2163322 m
    // b = 0.6055513 × 1.2163322 = 0.7365515 m
    // A = 2.1055513 × 1.2163322² = 3.1150872 m²
    // V = 2.0 / 3.1150872 = 0.6420366 m/s
    let freeboard = FreeboardTable::default();
    let design = design_channel(&base_input(&freeboard)).unwrap();

    assert!(close(design.flow_depth.value(), 1.216_332_2, 1e-6));
    assert!(close(design.bed_width.value(), 0.736_551_5, 1e-6));
    assert!(close(design.geometry().area.value(), 3.115_087_2, 1e-6));
    assert!(close(design.flow.velocity.value(), 0.642_036_6, 1e-6));
    assert!(close(design.computed_discharge.value(), 2.0, 1e-6));
    assert_eq!(design.flow.regime, FlowRegime::Subcritical);
    assert!(design.all_checks_pass());
}

#[test]
fn bed_width_to_depth_ratio_design_matches_hand_calculation() {
    // Q = 1.5 m³/s, S = 0.0005, m = 1.5, n = 0.025, b/h = k = 2
    // A = (k + m) h² = 3.5 h²
    // P = (k + 2 sqrt(1+m²)) h = (2 + 3.6055513) h = 5.6055513 h
    // A R^(2/3) = (k+m)^(5/3) / (k + 2 sqrt(1+m²))^(2/3) × h^(8/3)
    //   3.5^(5/3) = 8.1608, 5.6055513^(2/3) = 3.1917, c = 2.5568261
    // Q n / sqrt(S) = 1.5 × 0.025 / 0.0223607 = 1.6770510
    // h = (1.6770510 / 2.5568261)^(3/8) = 0.8537228 m
    // b = 2 × 0.8537228 = 1.7074457 m
    // A = 3.5 × 0.8537228² = 2.5509495 m², V = 1.5 / 2.5509495 = 0.5880164 m/s
    let freeboard = FreeboardTable::default();
    let design = design_channel(&ChannelDesignInput {
        design_discharge: Discharge::new(1.5).unwrap(),
        proportion: SectionProportion::BedWidthDepthRatio(2.0),
        ..base_input(&freeboard)
    })
    .unwrap();

    assert!(close(design.flow_depth.value(), 0.853_722_8, 1e-6));
    assert!(close(design.bed_width.value(), 1.707_445_7, 1e-6));
    assert!(close(design.bed_width_depth_ratio, 2.0, 1e-9));
    assert!(close(design.flow.velocity.value(), 0.588_016_4, 1e-6));
    assert!(close(design.computed_discharge.value(), 1.5, 1e-6));
    assert!(design.all_checks_pass());
}

#[test]
fn a_fixed_bed_width_is_solved_with_the_normal_depth_solver() {
    // b = 2.0 m fixed, Q = 2.0454609 m³/s, S = 0.0004, m = 1.5, n = 0.025.
    // The hand check in the uniform flow tests gives h = 1.0 m.
    let freeboard = FreeboardTable::default();
    let design = design_channel(&ChannelDesignInput {
        design_discharge: Discharge::new(2.045_460_9).unwrap(),
        bed_slope: Slope::new(0.0004).unwrap(),
        proportion: SectionProportion::FixedBedWidth(Length::new(2.0).unwrap()),
        ..base_input(&freeboard)
    })
    .unwrap();

    assert!(close(design.flow_depth.value(), 1.0, 1e-6));
    assert!(design.solver_iterations > 0);
    assert!(design.all_checks_pass());
}

#[test]
fn freeboard_and_total_depth_come_from_the_rule() {
    // Q = 2.0 m³/s falls in the 1.5 to 5 band of the convenience default table,
    // which gives 0.60 m. Total depth = 1.2163322 + 0.60 = 1.8163322 m.
    // Width at bank = b + 2 m (h + F) = 0.7365515 + 3 × 1.8163322 = 6.1855481 m
    let freeboard = FreeboardTable::default();
    let design = design_channel(&base_input(&freeboard)).unwrap();
    assert!(close(design.freeboard.value(), 0.60, 1e-12));
    assert!(close(design.total_depth.value(), 1.816_332_2, 1e-6));
    assert!(close(design.top_width_at_bank.value(), 6.185_548_1, 1e-6));
}

#[test]
fn the_efficient_shape_can_conflict_with_the_velocity_limit() {
    // The same canal on 1 in 100 instead of 1 in 2000.
    // Q n / sqrt(S) = 2.0 × 0.025 / 0.1 = 0.5
    // h = (0.5 / 1.3264142)^(3/8) = 0.6936000 m, b = 0.4200103 m
    // A = 2.1055513 × 0.6936² = 1.0129 m², V = 2.0 / 1.0129 = 1.9744495 m/s
    // That is nearly three times the 0.70 m/s permissible velocity for an
    // earth canal, and Fr = 0.9905 is on the edge of critical. The routine
    // still returns the section, with both failures recorded, because the
    // engineer needs to see the size of the conflict.
    let freeboard = FreeboardTable::default();
    let design = design_channel(&ChannelDesignInput {
        bed_slope: Slope::new(0.01).unwrap(),
        ..base_input(&freeboard)
    })
    .unwrap();

    assert!(close(design.flow_depth.value(), 0.693_600_0, 1e-6));
    assert!(close(design.flow.velocity.value(), 1.974_449_5, 1e-6));
    assert!(close(design.flow.froude_number, 0.990_512_4, 1e-6));
    assert!(!design.all_checks_pass());

    let failures = design.failures();
    assert_eq!(failures.len(), 2, "maximum velocity and Froude number");
    assert!(failures.iter().any(|c| c.name == "maximum velocity"));
    assert!(failures.iter().any(|c| c.name == "Froude number"));

    // The discharge is still reproduced exactly, so the failure is a design
    // conflict, not an arithmetic error.
    assert!(close(design.computed_discharge.value(), 2.0, 1e-6));
}

#[test]
fn a_supercritical_design_fails_the_flow_regime_check() {
    // A 1 in 20 lined chute carrying 2 m³/s at b/h = 2.
    let freeboard = FreeboardTable::default();
    let design = design_channel(&ChannelDesignInput {
        bed_slope: Slope::new(0.05).unwrap(),
        side_slope: SideSlope::new(0.0).unwrap(),
        roughness: ManningN::new(0.014).unwrap(),
        proportion: SectionProportion::BedWidthDepthRatio(2.0),
        velocity_limits: VelocityLimits::from_range(0.30, 3.0).unwrap(),
        ..base_input(&freeboard)
    })
    .unwrap();

    assert_eq!(design.flow.regime, FlowRegime::Supercritical);
    assert!(design.flow.froude_number > 1.0);
    let failures = design.failures();
    assert!(failures.iter().any(|c| c.name == "flow regime"));
}

#[test]
fn design_rejects_a_zero_slope() {
    let freeboard = FreeboardTable::default();
    let result = design_channel(&ChannelDesignInput {
        bed_slope: Slope::new(0.0).unwrap(),
        ..base_input(&freeboard)
    });
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "bed_slope",
            ..
        })
    ));
}

#[test]
fn the_default_freeboard_table_is_a_step_function_of_discharge() {
    let table = FreeboardTable::default();
    let at = |q: f64| table.freeboard(Discharge::new(q).unwrap()).unwrap().value();
    assert!(close(at(0.2), 0.40, 1e-12));
    assert!(close(at(0.5), 0.40, 1e-12), "band bounds are inclusive");
    assert!(close(at(0.51), 0.50, 1e-12));
    assert!(close(at(3.0), 0.60, 1e-12));
    assert!(close(at(8.0), 0.75, 1e-12));
    assert!(close(at(12.0), 0.85, 1e-12));
    assert!(close(at(40.0), 1.00, 1e-12), "above the last band");
    assert_eq!(table.steps().len(), 5);
}

#[test]
fn a_freeboard_table_without_an_open_top_rejects_a_discharge_above_it() {
    let table = FreeboardTable::new(
        vec![
            FreeboardStep::new(1.0, 0.40).unwrap(),
            FreeboardStep::new(5.0, 0.60).unwrap(),
        ],
        None,
    )
    .unwrap();
    assert!(close(
        table
            .freeboard(Discharge::new(4.0).unwrap())
            .unwrap()
            .value(),
        0.60,
        1e-12
    ));
    assert!(matches!(
        table.freeboard(Discharge::new(6.0).unwrap()),
        Err(KpError::TableOutOfRange { .. })
    ));
}

#[test]
fn a_freeboard_table_must_be_non_empty_and_ascending() {
    assert!(matches!(
        FreeboardTable::new(vec![], None),
        Err(KpError::EmptyTable { .. })
    ));
    assert!(matches!(
        FreeboardTable::new(
            vec![
                FreeboardStep::new(5.0, 0.60).unwrap(),
                FreeboardStep::new(1.0, 0.40).unwrap(),
            ],
            None,
        ),
        Err(KpError::OutOfRange { .. })
    ));
}

#[test]
fn a_freeboard_function_and_a_fixed_freeboard_are_both_usable_rules() {
    let curve = FreeboardFn::new(|q: Discharge| Length::new(0.35 + 0.05 * q.value().sqrt()));
    // Q = 4 m³/s: F = 0.35 + 0.05 × 2 = 0.45 m
    assert!(close(
        curve
            .freeboard(Discharge::new(4.0).unwrap())
            .unwrap()
            .value(),
        0.45,
        1e-12
    ));

    let fixed = FixedFreeboard(Length::new(0.55).unwrap());
    assert!(close(
        fixed
            .freeboard(Discharge::new(100.0).unwrap())
            .unwrap()
            .value(),
        0.55,
        1e-12
    ));
}

#[test]
fn a_project_minimum_freeboard_is_checked_against_the_rule() {
    // The default table gives 0.60 m at 2 m³/s. A project minimum of 0.80 m
    // is not met, and that is recorded rather than silently applied.
    let freeboard = FreeboardTable::default();
    let design = design_channel(&ChannelDesignInput {
        min_freeboard: Some(Length::new(0.80).unwrap()),
        ..base_input(&freeboard)
    })
    .unwrap();
    let check = design
        .checks
        .iter()
        .find(|c| c.name == "freeboard")
        .expect("freeboard check present");
    assert_eq!(check.status, CheckStatus::Fail);
    assert!(close(check.value, 0.60, 1e-12));
}

#[test]
fn the_design_report_names_every_value_an_engineer_checks() {
    let freeboard = FreeboardTable::default();
    let design = design_channel(&base_input(&freeboard)).unwrap();
    let report = design.to_string();
    for expected in [
        "design discharge",
        "bed width b",
        "flow depth h",
        "hydraulic radius R",
        "mean velocity V",
        "Froude number Fr",
        "freeboard F",
        "width at bank level",
        "Checks",
    ] {
        assert!(report.contains(expected), "report missing {expected}");
    }
}
