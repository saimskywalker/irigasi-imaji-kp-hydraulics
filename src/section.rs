//! Cross section geometry for open channels.
//!
//! Reference: KP-03, saluran irigasi (irrigation canals), cross section
//! definitions. Only the geometric relations are implemented here; the
//! criteria's tables of side slope against soil type and of bed width to depth
//! ratio against discharge are lookups the engineer reads off the criteria and
//! passes in as parameters.
//!
//! # Units
//!
//! All lengths are metres, areas are square metres, side slopes are metres
//! horizontal per metre vertical. See [`crate::units`].
//!
//! Sign convention for the trapezoid: `b` is the bed width, `h` the flow depth,
//! `m` the side slope. Then
//!
//! ```text
//! A = (b + m h) h
//! P = b + 2 h sqrt(1 + m²)
//! T = b + 2 m h
//! R = A / P
//! D = A / T          (hydraulic depth, used in the Froude number)
//! ```

use crate::error::{positive, Result};
use crate::units::{Area, Length, SideSlope};

/// Every geometric property of a section at one flow depth.
///
/// Returned as a group because a design report wants all of them, and
/// recomputing them separately invites inconsistency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionGeometry {
    /// Flow depth, m.
    pub depth: Length,
    /// Flow area, m².
    pub area: Area,
    /// Wetted perimeter, m.
    pub wetted_perimeter: Length,
    /// Hydraulic radius A/P, m.
    pub hydraulic_radius: Length,
    /// Water surface width, m.
    pub top_width: Length,
    /// Hydraulic depth A/T, m. This is the length scale in the Froude number,
    /// not the flow depth, and the two differ for any non-rectangular section.
    pub hydraulic_depth: Length,
}

/// A channel cross section whose geometry can be evaluated at a flow depth.
///
/// Implemented by [`Trapezoid`], [`Rectangle`] and [`Triangle`]. The trait is
/// object safe, so a design routine can take `&dyn Section`.
pub trait Section {
    /// Flow area at the given depth, m².
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError`] if the depth is zero or the geometry is
    /// degenerate.
    fn flow_area(&self, depth: Length) -> Result<Area>;

    /// Wetted perimeter at the given depth, m.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError`] if the depth is zero or the geometry is
    /// degenerate.
    fn wetted_perimeter(&self, depth: Length) -> Result<Length>;

    /// Water surface width at the given depth, m.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError`] if the depth is zero or the geometry is
    /// degenerate.
    fn top_width(&self, depth: Length) -> Result<Length>;

    /// Hydraulic radius A/P at the given depth, m.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError`] if the wetted perimeter is zero, which can
    /// only happen for a degenerate section.
    fn hydraulic_radius(&self, depth: Length) -> Result<Length> {
        let area = self.flow_area(depth)?;
        let perimeter = positive("wetted_perimeter", self.wetted_perimeter(depth)?.value())?;
        Length::new(area.value() / perimeter)
    }

    /// Hydraulic depth A/T at the given depth, m.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError`] if the top width is zero.
    fn hydraulic_depth(&self, depth: Length) -> Result<Length> {
        let area = self.flow_area(depth)?;
        let top_width = positive("top_width", self.top_width(depth)?.value())?;
        Length::new(area.value() / top_width)
    }

    /// Evaluates every geometric property at once.
    ///
    /// # Errors
    ///
    /// Returns [`crate::KpError`] if any individual property cannot be
    /// computed.
    fn geometry(&self, depth: Length) -> Result<SectionGeometry> {
        Ok(SectionGeometry {
            depth,
            area: self.flow_area(depth)?,
            wetted_perimeter: self.wetted_perimeter(depth)?,
            hydraulic_radius: self.hydraulic_radius(depth)?,
            top_width: self.top_width(depth)?,
            hydraulic_depth: self.hydraulic_depth(depth)?,
        })
    }
}

/// A trapezoidal section, the standard shape for an Indonesian irrigation
/// canal, KP-03.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trapezoid {
    /// Bed width b, m.
    pub bed_width: Length,
    /// Side slope m, metres horizontal per metre vertical.
    pub side_slope: SideSlope,
}

impl Trapezoid {
    /// Creates a trapezoidal section.
    ///
    /// # Examples
    ///
    /// ```
    /// use kp_irigasi::section::{Section, Trapezoid};
    /// use kp_irigasi::units::{Length, SideSlope};
    ///
    /// // b = 2.0 m, m = 1.5, h = 1.0 m
    /// // A = (2.0 + 1.5 × 1.0) × 1.0 = 3.5 m²
    /// let section = Trapezoid::new(Length::new(2.0)?, SideSlope::new(1.5)?);
    /// let area = section.flow_area(Length::new(1.0)?)?;
    /// assert!((area.value() - 3.5).abs() < 1e-12);
    /// # Ok::<(), kp_irigasi::KpError>(())
    /// ```
    pub const fn new(bed_width: Length, side_slope: SideSlope) -> Self {
        Self {
            bed_width,
            side_slope,
        }
    }
}

impl Section for Trapezoid {
    fn flow_area(&self, depth: Length) -> Result<Area> {
        let h = positive("depth", depth.value())?;
        let b = self.bed_width.value();
        let m = self.side_slope.value();
        Area::new((b + m * h) * h)
    }

    fn wetted_perimeter(&self, depth: Length) -> Result<Length> {
        let h = positive("depth", depth.value())?;
        let b = self.bed_width.value();
        Length::new(b + 2.0 * h * self.side_slope.batter_factor())
    }

    fn top_width(&self, depth: Length) -> Result<Length> {
        let h = positive("depth", depth.value())?;
        let b = self.bed_width.value();
        let m = self.side_slope.value();
        Length::new(b + 2.0 * m * h)
    }
}

/// A rectangular section, that is a trapezoid with vertical walls.
///
/// Used for flumes, chutes and lined structures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    /// Width b, m.
    pub width: Length,
}

impl Rectangle {
    /// Creates a rectangular section.
    ///
    /// # Examples
    ///
    /// ```
    /// use kp_irigasi::section::{Rectangle, Section};
    /// use kp_irigasi::units::Length;
    ///
    /// // b = 1.2 m, h = 0.6 m: A = 0.72 m², P = 1.2 + 2 × 0.6 = 2.4 m
    /// let flume = Rectangle::new(Length::new(1.2)?);
    /// let g = flume.geometry(Length::new(0.6)?)?;
    /// assert!((g.area.value() - 0.72).abs() < 1e-12);
    /// assert!((g.wetted_perimeter.value() - 2.4).abs() < 1e-12);
    /// # Ok::<(), kp_irigasi::KpError>(())
    /// ```
    pub const fn new(width: Length) -> Self {
        Self { width }
    }
}

impl Section for Rectangle {
    fn flow_area(&self, depth: Length) -> Result<Area> {
        let h = positive("depth", depth.value())?;
        Area::new(self.width.value() * h)
    }

    fn wetted_perimeter(&self, depth: Length) -> Result<Length> {
        let h = positive("depth", depth.value())?;
        Length::new(self.width.value() + 2.0 * h)
    }

    fn top_width(&self, _depth: Length) -> Result<Length> {
        Ok(self.width)
    }
}

/// A triangular section, that is a trapezoid with zero bed width.
///
/// Occurs in small field drains and in the V shaped bed of a silted canal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    /// Side slope m, metres horizontal per metre vertical, on both banks.
    pub side_slope: SideSlope,
}

impl Triangle {
    /// Creates a triangular section.
    ///
    /// # Examples
    ///
    /// ```
    /// use kp_irigasi::section::{Section, Triangle};
    /// use kp_irigasi::units::{Length, SideSlope};
    ///
    /// // m = 1.0, h = 0.5 m: A = 1.0 × 0.5² = 0.25 m², T = 2 × 1.0 × 0.5 = 1.0 m
    /// let drain = Triangle::new(SideSlope::new(1.0)?);
    /// let g = drain.geometry(Length::new(0.5)?)?;
    /// assert!((g.area.value() - 0.25).abs() < 1e-12);
    /// assert!((g.top_width.value() - 1.0).abs() < 1e-12);
    /// # Ok::<(), kp_irigasi::KpError>(())
    /// ```
    pub const fn new(side_slope: SideSlope) -> Self {
        Self { side_slope }
    }
}

impl Section for Triangle {
    fn flow_area(&self, depth: Length) -> Result<Area> {
        let h = positive("depth", depth.value())?;
        Area::new(self.side_slope.value() * h * h)
    }

    fn wetted_perimeter(&self, depth: Length) -> Result<Length> {
        let h = positive("depth", depth.value())?;
        Length::new(2.0 * h * self.side_slope.batter_factor())
    }

    fn top_width(&self, depth: Length) -> Result<Length> {
        let h = positive("depth", depth.value())?;
        Length::new(2.0 * self.side_slope.value() * h)
    }
}
