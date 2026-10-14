//! Worked examples for section geometry and uniform flow.
//!
//! Every expected value is hand calculated in the comment above the assertion,
//! using g = 9.81 m/s². Units follow the crate convention: m, m², m/s, m³/s.

use imaji_kp_hydraulics::error::KpError;
use imaji_kp_hydraulics::section::{Rectangle, Section, Trapezoid, Triangle};
use imaji_kp_hydraulics::uniform_flow::{
    froude_number, normal_depth, uniform_flow, FlowRegime, NormalDepthOptions, VelocityLimits,
};
use imaji_kp_hydraulics::units::{Discharge, Length, ManningN, SideSlope, Slope, Velocity};

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

#[test]
fn trapezoid_geometry_matches_hand_calculation() {
    // b = 1.5 m, m = 1.0, h = 0.8 m
    // A = (1.5 + 1.0 × 0.8) × 0.8 = 2.3 × 0.8 = 1.84 m²
    // P = 1.5 + 2 × 0.8 × sqrt(1 + 1) = 1.5 + 1.6 × 1.4142136 = 3.7627417 m
    // R = 1.84 / 3.7627417 = 0.4890051 m
    // T = 1.5 + 2 × 1.0 × 0.8 = 3.1 m
    // D = 1.84 / 3.1 = 0.5935484 m
    let section = Trapezoid::new(Length::new(1.5).unwrap(), SideSlope::new(1.0).unwrap());
    let g = section.geometry(Length::new(0.8).unwrap()).unwrap();
    assert!(close(g.area.value(), 1.84, 1e-12));
    assert!(close(g.wetted_perimeter.value(), 3.762_741_7, 1e-7));
    assert!(close(g.hydraulic_radius.value(), 0.489_005_1, 1e-7));
    assert!(close(g.top_width.value(), 3.1, 1e-12));
    assert!(close(g.hydraulic_depth.value(), 0.593_548_4, 1e-7));
}

#[test]
fn rectangle_geometry_matches_hand_calculation() {
    // b = 1.2 m, h = 0.6 m
    // A = 0.72 m², P = 1.2 + 1.2 = 2.4 m, R = 0.30 m
    // For a rectangle the hydraulic depth D = A/T equals the flow depth.
    let section = Rectangle::new(Length::new(1.2).unwrap());
    let g = section.geometry(Length::new(0.6).unwrap()).unwrap();
    assert!(close(g.area.value(), 0.72, 1e-12));
    assert!(close(g.wetted_perimeter.value(), 2.4, 1e-12));
    assert!(close(g.hydraulic_radius.value(), 0.30, 1e-12));
    assert!(close(g.hydraulic_depth.value(), 0.60, 1e-12));
}

#[test]
fn triangle_geometry_matches_hand_calculation() {
    // m = 1.0, h = 0.5 m
    // A = m h² = 0.25 m², P = 2 h sqrt(1 + m²) = 2 × 0.5 × sqrt(2) = 1.4142136 m
    // R = 0.25 / 1.4142136 = 0.1767767 m, T = 2 m h = 1.0 m, D = 0.25 m
    let section = Triangle::new(SideSlope::new(1.0).unwrap());
    let g = section.geometry(Length::new(0.5).unwrap()).unwrap();
    assert!(close(g.area.value(), 0.25, 1e-12));
    assert!(close(
        g.wetted_perimeter.value(),
        std::f64::consts::SQRT_2,
        1e-7
    ));
    assert!(close(g.hydraulic_radius.value(), 0.176_776_7, 1e-7));
    assert!(close(g.hydraulic_depth.value(), 0.25, 1e-12));
}

#[test]
fn hydraulic_depth_is_not_the_flow_depth_for_a_trapezoid() {
    // A trap for the unwary: D = A/T = 0.5935 m against a flow depth of 0.8 m.
    // Using the flow depth in the Froude number would understate Fr by 16 per
    // cent here.
    let section = Trapezoid::new(Length::new(1.5).unwrap(), SideSlope::new(1.0).unwrap());
    let g = section.geometry(Length::new(0.8).unwrap()).unwrap();
    assert!(g.hydraulic_depth.value() < g.depth.value());
    assert!(close(g.hydraulic_depth.value(), 0.593_548_4, 1e-7));
}

#[test]
fn manning_discharge_matches_hand_calculation() {
    // b = 1.5 m, m = 1.0, h = 0.8 m, n = 0.020, S = 0.001
    // A = 1.84 m², R = 0.4890051 m
    // R^(2/3) = 0.6239378
    // V = (1/0.020) × 0.6239378 × sqrt(0.001) = 50 × 0.6239378 × 0.0316228
    //   = 0.9813990 m/s
    // Q = 1.84 × 0.9813990 = 1.8057742 m³/s
    // Fr = 0.9813990 / sqrt(9.81 × 0.5935484) = 0.9813990 / 2.4130 = 0.4067083
    let section = Trapezoid::new(Length::new(1.5).unwrap(), SideSlope::new(1.0).unwrap());
    let flow = uniform_flow(
        &section,
        Length::new(0.8).unwrap(),
        ManningN::new(0.020).unwrap(),
        Slope::new(0.001).unwrap(),
    )
    .unwrap();
    assert!(close(flow.velocity.value(), 0.981_399_0, 1e-7));
    assert!(close(flow.discharge.value(), 1.805_774_2, 1e-7));
    assert!(close(flow.froude_number, 0.406_708_3, 1e-7));
    assert_eq!(flow.regime, FlowRegime::Subcritical);
}

#[test]
fn velocity_is_consistent_with_discharge_over_area() {
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let flow = uniform_flow(
        &section,
        Length::new(1.1).unwrap(),
        ManningN::new(0.025).unwrap(),
        Slope::new(0.0004).unwrap(),
    )
    .unwrap();
    let implied = flow.discharge.value() / flow.geometry.area.value();
    assert!(close(flow.velocity.value(), implied, 1e-12));
}

#[test]
fn conveyance_scales_with_the_square_root_of_slope() {
    // Q = K sqrt(S), so quadrupling the slope doubles the discharge for the
    // same depth.
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let n = ManningN::new(0.025).unwrap();
    let depth = Length::new(1.0).unwrap();
    let shallow = uniform_flow(&section, depth, n, Slope::new(0.0004).unwrap()).unwrap();
    let steep = uniform_flow(&section, depth, n, Slope::new(0.0016).unwrap()).unwrap();
    assert!(close(
        steep.discharge.value(),
        2.0 * shallow.discharge.value(),
        1e-9
    ));
    assert!(close(shallow.conveyance, steep.conveyance, 1e-9));
}

#[test]
fn zero_slope_is_rejected() {
    // A level bed is a valid geometry but Manning's equation has no solution on
    // it, so the crate refuses rather than dividing by zero.
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let result = uniform_flow(
        &section,
        Length::new(1.0).unwrap(),
        ManningN::new(0.025).unwrap(),
        Slope::new(0.0).unwrap(),
    );
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "slope",
            ..
        })
    ));
}

#[test]
fn zero_depth_is_rejected() {
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let result = section.flow_area(Length::new(0.0).unwrap());
    assert!(matches!(
        result,
        Err(KpError::NotPositive {
            parameter: "depth",
            ..
        })
    ));
}

#[test]
fn negative_and_non_finite_inputs_are_rejected_at_construction() {
    assert!(matches!(
        Length::new(-0.1),
        Err(KpError::Negative {
            parameter: "Length",
            ..
        })
    ));
    assert!(matches!(
        Discharge::new(f64::NAN),
        Err(KpError::NotFinite {
            parameter: "Discharge",
            ..
        })
    ));
    assert!(matches!(
        ManningN::new(0.0),
        Err(KpError::NotPositive {
            parameter: "ManningN",
            ..
        })
    ));
}

#[test]
fn normal_depth_round_trips_against_manning() {
    // Compute the discharge at h = 1.0 m, then solve for the depth again.
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let n = ManningN::new(0.025).unwrap();
    let s = Slope::new(0.0004).unwrap();
    let q = uniform_flow(&section, Length::new(1.0).unwrap(), n, s)
        .unwrap()
        .discharge;
    let solved = normal_depth(&section, q, n, s, NormalDepthOptions::default()).unwrap();
    assert!(close(solved.depth.value(), 1.0, 1e-6));
    assert!(solved.discharge_residual.abs() < 1e-6);
    assert!(solved.iterations > 0);
    assert!(solved.iterations <= 100);
}

#[test]
fn normal_depth_matches_hand_calculation() {
    // b = 2.0 m, m = 1.5, n = 0.025, S = 0.0004, Q = 2.045461 m³/s.
    // Hand check at h = 1.0 m:
    //   A = 3.5 m², P = 5.6055513 m, R = 0.6243811 m
    //   V = 40 × 0.6243811^(2/3) × 0.02 = 40 × 0.7305217 × 0.02 = 0.5844174 m/s
    //   Q = 3.5 × 0.5844174 = 2.0454609 m³/s, so the normal depth is 1.0 m.
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let solved = normal_depth(
        &section,
        Discharge::new(2.045_460_9).unwrap(),
        ManningN::new(0.025).unwrap(),
        Slope::new(0.0004).unwrap(),
        NormalDepthOptions::default(),
    )
    .unwrap();
    assert!(close(solved.depth.value(), 1.0, 1e-6));
}

#[test]
fn normal_depth_reports_non_convergence_rather_than_guessing() {
    // Three bisection steps cannot narrow the bracket to 1e-6 m, so the solver
    // must fail loudly instead of returning a half solved depth.
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let options = NormalDepthOptions {
        max_iterations: 3,
        ..NormalDepthOptions::default()
    };
    let result = normal_depth(
        &section,
        Discharge::new(2.0).unwrap(),
        ManningN::new(0.025).unwrap(),
        Slope::new(0.0004).unwrap(),
        options,
    );
    match result {
        Err(KpError::DidNotConverge {
            routine,
            iterations,
            tolerance,
            residual,
        }) => {
            assert_eq!(routine, "normal_depth");
            assert_eq!(iterations, 3);
            assert!(close(tolerance, 1e-6, 1e-18));
            assert!(residual > tolerance);
        }
        other => panic!("expected DidNotConverge, got {other:?}"),
    }
}

#[test]
fn normal_depth_reports_no_bracket_when_the_section_is_too_small() {
    // A 2 m wide canal on 1 in 2500 cannot carry 500 m³/s at any depth below
    // the 3 m search limit.
    let section = Trapezoid::new(Length::new(2.0).unwrap(), SideSlope::new(1.5).unwrap());
    let options = NormalDepthOptions {
        max_depth: 3.0,
        ..NormalDepthOptions::default()
    };
    let result = normal_depth(
        &section,
        Discharge::new(500.0).unwrap(),
        ManningN::new(0.025).unwrap(),
        Slope::new(0.0004).unwrap(),
        options,
    );
    assert!(matches!(
        result,
        Err(KpError::NoBracket {
            routine: "normal_depth",
            ..
        })
    ));
}

#[test]
fn froude_number_matches_hand_calculation() {
    // V = 0.6 m/s, D = 0.8 m: Fr = 0.6 / sqrt(9.81 × 0.8) = 0.6 / 2.8014282
    //    = 0.2141765
    let fr = froude_number(Velocity::new(0.6).unwrap(), Length::new(0.8).unwrap()).unwrap();
    assert!(close(fr, 0.214_176_5, 1e-7));
}

#[test]
fn flow_regime_classification_covers_all_three_states() {
    assert_eq!(FlowRegime::classify(0.45).unwrap(), FlowRegime::Subcritical);
    assert_eq!(FlowRegime::classify(1.0).unwrap(), FlowRegime::Critical);
    assert_eq!(
        FlowRegime::classify(1.0005).unwrap(),
        FlowRegime::Critical,
        "inside the default critical band"
    );
    assert_eq!(
        FlowRegime::classify(2.6).unwrap(),
        FlowRegime::Supercritical
    );
    assert!(FlowRegime::is_near_critical(0.95, 0.1));
    assert!(!FlowRegime::is_near_critical(0.5, 0.1));
}

#[test]
fn a_steep_lined_chute_runs_supercritical() {
    // b = 1.0 m rectangular chute, n = 0.014, S = 0.05, Q = 2.0 m³/s.
    // Normal depth solves to 0.3565465 m, giving
    //   A = 0.3565465 m², P = 1.7130930 m, R = 0.2081376 m
    //   V = (1/0.014) × 0.2081376^(2/3) × sqrt(0.05) = 5.6093663 m/s
    //   Fr = 5.6093663 / sqrt(9.81 × 0.3565465) = 2.9993089
    let chute = Rectangle::new(Length::new(1.0).unwrap());
    let solved = normal_depth(
        &chute,
        Discharge::new(2.0).unwrap(),
        ManningN::new(0.014).unwrap(),
        Slope::new(0.05).unwrap(),
        NormalDepthOptions::default(),
    )
    .unwrap();
    assert!(close(solved.depth.value(), 0.356_546_5, 1e-6));
    assert!(close(solved.flow.velocity.value(), 5.609_366_3, 1e-5));
    assert!(close(solved.flow.froude_number, 2.999_308_9, 1e-5));
    assert_eq!(solved.flow.regime, FlowRegime::Supercritical);
}

#[test]
fn velocity_limits_produce_a_pass_and_a_fail() {
    use imaji_kp_hydraulics::check::{CheckList, CheckStatus};

    let limits = VelocityLimits::from_range(0.30, 0.70).unwrap();

    let ok = limits.check(Velocity::new(0.58).unwrap());
    assert!(ok.all_pass());

    let too_slow = limits.check(Velocity::new(0.12).unwrap());
    assert_eq!(too_slow[0].status, CheckStatus::Fail);
    assert_eq!(too_slow[1].status, CheckStatus::Pass);

    let too_fast = limits.check(Velocity::new(1.20).unwrap());
    assert_eq!(too_fast[0].status, CheckStatus::Pass);
    assert_eq!(too_fast[1].status, CheckStatus::Fail);
}

#[test]
fn an_omitted_velocity_limit_is_recorded_as_not_checked() {
    use imaji_kp_hydraulics::check::{CheckList, CheckStatus};

    let limits = VelocityLimits::new(None, Some(Velocity::new(0.70).unwrap()));
    let checks = limits.check(Velocity::new(0.05).unwrap());
    assert_eq!(checks[0].status, CheckStatus::NotChecked);
    assert!(
        checks.all_pass(),
        "a criterion the caller did not apply is not a failure"
    );
}
