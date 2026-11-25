//! Persistent material-space pattern measurements.
//!
//! Measurements belong to named panel boundaries and use sampled-boundary
//! arclength fractions. They are nonphysical metadata and never reconstruct or
//! alter rest geometry, meshes, placement or solver state.

use super::polygon::PolygonPattern;
use glam::DVec2;
use serde::{Deserialize, Serialize};

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurementStation {
    pub panel_id: String,
    pub boundary_id: String,
    pub fraction: f64,
}

impl MeasurementStation {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.panel_id) {
            return Err("Measurement panel ID is invalid".into());
        }
        if !valid_id(&self.boundary_id) {
            return Err("Measurement boundary ID is invalid".into());
        }
        if !self.fraction.is_finite() || !(0.0..=1.0).contains(&self.fraction) {
            return Err("Measurement station fraction must be between 0 and 1".into());
        }
        Ok(())
    }

    pub fn point(&self, polygon: &PolygonPattern) -> Result<DVec2, String> {
        self.validate()?;
        let edge = polygon
            .edges
            .iter()
            .position(|edge| edge == &self.boundary_id)
            .ok_or_else(|| "Measurement references a missing boundary".to_owned())?;
        let samples = polygon.edge_points(edge)?;
        if samples.len() < 2 {
            return Err("Measurement boundary is degenerate".into());
        }
        let mut lengths = Vec::with_capacity(samples.len());
        lengths.push(0.0);
        let mut total = 0.0;
        for pair in samples.windows(2) {
            total += pair[0].distance(pair[1]);
            lengths.push(total);
        }
        if total <= 1e-9 {
            return Err("Measurement boundary is too short".into());
        }
        let target = self.fraction * total;
        if target <= 0.0 {
            return Ok(samples[0]);
        }
        if target >= total {
            return Ok(*samples.last().unwrap());
        }
        for (i, pair) in samples.windows(2).enumerate() {
            let segment_start = lengths[i];
            let segment_end = lengths[i + 1];
            if target <= segment_end {
                let span = segment_end - segment_start;
                if span <= 0.0 {
                    continue;
                }
                let t = (target - segment_start) / span;
                return Ok(pair[0] + (pair[1] - pair[0]) * t);
            }
        }
        Ok(*samples.last().unwrap())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternMeasurement {
    pub id: String,
    pub label: String,
    pub from: MeasurementStation,
    pub to: MeasurementStation,
}

impl PatternMeasurement {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("Pattern measurement ID is invalid".into());
        }
        if self.label.trim().is_empty() || self.label.chars().count() > 120 {
            return Err("Pattern measurement label must contain 1–120 characters".into());
        }
        self.from.validate()?;
        self.to.validate()?;
        if self.from == self.to {
            return Err("Pattern measurement endpoints must differ".into());
        }
        Ok(())
    }
}
