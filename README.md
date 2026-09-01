# kp-irigasi

Irrigation design calculations following the Indonesian Kriteria Perencanaan
(KP) irrigation planning criteria, KP-01 to KP-07, published by the Direktorat
Jenderal Sumber Daya Air, Kementerian Pekerjaan Umum dan Perumahan Rakyat.

The crate implements the equations and the procedures, and cites the KP part
each one belongs to. It does not reproduce the text of the criteria and it does
not embed their design tables. Where a criterion is a value the engineer reads
off a chart or a table, freeboard against discharge, minimum side slope against
soil type, maximum permissible velocity against bed material, that value is a
parameter of the function or a caller-supplied table. A few types carry a
`Default` with a handful of widely published representative values so that the
examples run without project data to hand; each is documented as a convenience
the engineer must confirm against the current KP edition.


## Why this exists

Engineering calculation in Indonesia largely runs on spreadsheets passed
between offices by email. No version control, no tests, no audit trail, and no
way to tell whether the copy you were sent still matches the one that was
checked. A library with a test suite is a modest improvement, but a real one:
the arithmetic can be reviewed, the assumptions are written down, and a change
that breaks a result fails loudly.

For steel and concrete members, load combinations and seismic base shear, see
[sni-struct](https://github.com/saimskywalker/sni-struct). Structural design and
hydraulic design are different enough in practice that they are kept apart.

## Disclaimer

Everything here is a calculation aid. It is not a substitute for a licensed
engineer's judgement, and no result from it should be built without being
checked against the governing criteria and against the conditions of the site.

## What it covers

| Area | Module | KP part |
|---|---|---|
| Manning's equation both ways, normal depth, Froude number, flow regime, velocity limits | `uniform_flow` | KP-03 |
| Trapezoidal, rectangular and triangular section geometry | `section` | KP-03 |
| Freeboard as a caller-supplied table or function | `freeboard` | KP-03 |
| Most efficient trapezoid, full canal design routine with all checks | `design` | KP-03 |
| Net field requirement, land preparation, efficiency chain to the intake, short supply | `water` | KP-01 |
| Tertiary and quaternary block sizing and layout limits | `tertiary` | KP-01, KP-07 |
| Weir, orifice and gate discharge, measuring weirs, hydraulic jump, straight drop | `structures` | KP-02, KP-04 |
| Culvert and siphon head loss and barrel sizing | `conduit` | KP-04 |

## Unit convention

SI throughout, with the two exceptions Indonesian irrigation practice insists
on: depth rates in mm/day and areas in hectares. Every quantity is a distinct
newtype, so the compiler rejects a discharge passed where a depth rate is
wanted, and every conversion is an explicitly named method.

| Quantity | Type | Unit |
|---|---|---|
| Length, depth, width, head, freeboard | `Length` | m |
| Flow area, cross section area | `Area` | m² |
| Velocity | `Velocity` | m/s |
| Discharge | `Discharge` | m³/s |
| Bed slope, energy slope | `Slope` | m/m |
| Side slope | `SideSlope` | m horizontal per 1 m vertical |
| Manning roughness | `ManningN` | s·m^(-1/3) |
| Dimensionless ratio, efficiency, coefficient | `Ratio` | none |
| Evapotranspiration, percolation, rainfall | `DepthRate` | mm/day |
| Standing water layer | `WaterLayer` | mm |
| Irrigated area | `Hectares` | ha |
| Period | `Days` | day |

Irrigation practice mixes litres per second, m³/s and mm/day freely, and a
silent unit mix-up is the classic way to get an irrigation calculation wrong.
So `Discharge::from_litres_per_second` and `DepthRate::over_area` are the only
routes between those units, and both are named at the call site. The
mm/day to l/s/ha divisor 8.64 appears in one place in the crate.

Every constructor validates. `NaN`, infinity, and a negative value for a
quantity that cannot be negative are rejected with a `KpError`, so a bad number
cannot travel silently into a design.

## Usage

```rust
use kp_irigasi::design::{design_channel, ChannelDesignInput, SectionProportion};
use kp_irigasi::freeboard::FreeboardTable;
use kp_irigasi::uniform_flow::{NormalDepthOptions, VelocityLimits};
use kp_irigasi::units::{Discharge, ManningN, SideSlope, Slope};

fn main() -> Result<(), kp_irigasi::KpError> {
    let freeboard = FreeboardTable::default();
    let design = design_channel(&ChannelDesignInput {
        design_discharge: Discharge::new(1.85)?,
        bed_slope: Slope::one_in(2500.0)?,
        side_slope: SideSlope::new(1.5)?,
        roughness: ManningN::new(0.025)?,
        proportion: SectionProportion::BedWidthDepthRatio(2.0),
        velocity_limits: VelocityLimits::from_range(0.25, 0.70)?,
        freeboard_rule: &freeboard,
        max_froude: Some(0.7),
        min_freeboard: None,
        solver: NormalDepthOptions::default(),
    })?;

    assert!(design.all_checks_pass());
    println!("{design}");
    Ok(())
}
```

Every design routine returns a rich result, not a bare number: the geometry,
the velocity, the Froude number and regime, the freeboard, the intermediate
values, and every check with its pass or fail. Each result type implements
`Display` and prints a readable calculation report.

## Design rules

- No panics in library code. Every fallible operation returns `Result` with a
  descriptive `KpError`. `unwrap` and `expect` appear only in tests and
  examples.
- Iterative solvers are bounded and honest. Normal depth and barrel sizing both
  bisect, with a documented tolerance and iteration limit, and return
  `KpError::DidNotConverge` or `KpError::NoBracket` rather than looping or
  handing back an unconverged answer.
- A failed check does not stop a calculation. The section is returned with the
  failure recorded, because the engineer needs to see by how much a limit is
  exceeded in order to decide what to change.
- No dependencies beyond the standard library.

## Examples

```text
cargo run -p kp-irigasi --example trapezoidal_channel
cargo run -p kp-irigasi --example intake_requirement
cargo run -p kp-irigasi --example drop_and_siphon
```

`trapezoidal_channel` sizes a secondary canal three ways and compares them.
`intake_requirement` works a water requirement from the field to the intake and
checks the tertiary layout. `drop_and_siphon` sizes a drop basin and a siphon
barrel.

## Terms

Indonesian terms are used where they are the accepted name, with an English
gloss on first use: petak tersier (tertiary block), petak kuarter (quaternary
block), saluran primer (primary canal), saluran sekunder (secondary canal),
saluran tersier (tertiary channel), bangunan bagi (division structure),
bangunan sadap (offtake structure), bangunan terjun (drop structure), tinggi
jagaan (freeboard), golongan (rotation group).

## Licence

MIT or Apache-2.0, at your option.
