//! Read-only board measurements. These points are presentation coordinates,
//! not persistent garment constraints or distances on the draped surface.
use super::*;
use glam::DVec2;

#[derive(Default)]
pub(super) struct Tape {
    start: Option<DVec2>,
    end: Option<DVec2>,
}
impl Tape {
    fn click(&mut self, point: DVec2) {
        if self.start.is_none() || self.end.is_some() {
            self.start = Some(point);
            self.end = None;
        } else {
            self.end = Some(point);
        }
    }
    pub(super) fn segment(&self, hover: Option<DVec2>) -> Option<[DVec2; 2]> {
        Some([self.start?, self.end.or(hover)?])
    }
    pub(super) fn has_draft(&self) -> bool {
        self.start.is_some() || self.end.is_some()
    }
}

pub(super) fn station_point(
    panel_id: &str,
    polygon: &marvelous_cloth::garment::polygon::PolygonPattern,
    station: &marvelous_cloth::garment::measurements::MeasurementStation,
) -> Result<DVec2, String> {
    if station.panel_id != panel_id {
        return Err("Measurement station belongs to a different panel".into());
    }
    station.point(polygon)
}

pub(super) fn station_for_point(
    panel_id: &str,
    polygon: &marvelous_cloth::garment::polygon::PolygonPattern,
    point: DVec2,
) -> Result<marvelous_cloth::garment::measurements::MeasurementStation, String> {
    use marvelous_cloth::garment::measurements::MeasurementStation;

    let mut best: Option<(f64, usize, f64, f64)> = None;
    for edge in 0..polygon.edges.len() {
        let samples = polygon.edge_points(edge)?;
        let mut segment_start = 0.0;
        let mut lengths = Vec::with_capacity(samples.len());
        lengths.push(0.0);
        for pair in samples.windows(2) {
            segment_start += pair[0].distance(pair[1]);
            lengths.push(segment_start);
        }
        if segment_start <= 1e-9 {
            continue;
        }
        for (i, pair) in samples.windows(2).enumerate() {
            let delta = pair[1] - pair[0];
            let span = delta.length_squared();
            if span <= 1e-18 {
                continue;
            }
            let t = ((point - pair[0]).dot(delta) / span).clamp(0.0, 1.0);
            let projected = pair[0] + delta * t;
            let distance = point.distance(projected);
            let before = lengths[i];
            let station = (before + t * (lengths[i + 1] - before)) / segment_start;
            if best.is_none_or(|current| distance < current.0) {
                best = Some((distance, edge, station, lengths[i + 1] - before));
            }
        }
    }
    let Some((_, _, fraction, _)) = best else {
        return Err("Point is not near a named panel boundary".into());
    };
    let edge = best.map(|(_, edge, _, _)| edge).unwrap();
    Ok(MeasurementStation {
        panel_id: panel_id.into(),
        boundary_id: polygon.edges[edge].clone(),
        fraction,
    })
}
impl Atelier {
    pub(super) fn measure_board(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        response: &egui::Response,
    ) {
        let hover = self.authoring.board.hover_snap.map(|s| s.point);
        if response.clicked()
            && let Some(point) = hover
        {
            self.authoring.measure.click(point);
            self.authoring.pattern_measurement_dirty = true;
        }
        if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.authoring.measure = Default::default();
        }
        let painter = ui.painter().with_clip_rect(rect);

        if let Some(panel_id) = self.authoring.selected.clone()
            && let Some(panel_index) = self
                .project
                .template
                .panels
                .iter()
                .position(|panel| panel.id == panel_id)
        {
            let panel = &self.project.template.panels[panel_index];
            if let Some(polygon) = panel.polygon.as_ref() {
                let origin =
                    marvelous_cloth::authoring::layout(&self.project.template, panel_index);
                for measurement in &self.project.template.measurements {
                    if measurement.from.panel_id != panel_id {
                        continue;
                    }
                    if let (Ok(a), Ok(b)) = (
                        station_point(&panel_id, polygon, &measurement.from),
                        station_point(&panel_id, polygon, &measurement.to),
                    ) {
                        let board_a = origin
                            + marvelous_cloth::authoring::rotate_2d(
                                a,
                                panel.layout_rotation_degrees,
                            );
                        let board_b = origin
                            + marvelous_cloth::authoring::rotate_2d(
                                b,
                                panel.layout_rotation_degrees,
                            );
                        let a = self.authoring.board.screen(rect, board_a);
                        let b = self.authoring.board.screen(rect, board_b);
                        let selected = self.authoring.pattern_measurement_id.as_deref()
                            == Some(measurement.id.as_str());
                        let color = if selected {
                            ui.visuals().selection.stroke.color
                        } else {
                            egui::Color32::from_rgb(0x15, 0x65, 0xC0)
                        };
                        painter.line_segment(
                            [a, b],
                            egui::Stroke::new(if selected { 3. } else { 2. }, color),
                        );
                        for point in [a, b] {
                            painter.circle_filled(point, 4., color);
                        }
                        let distance_mm = board_a.distance(board_b) * 1000.;
                        let midpoint = egui::pos2((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
                        painter.text(
                            midpoint,
                            egui::Align2::CENTER_BOTTOM,
                            format!("{} · {:.1} mm", measurement.label, distance_mm),
                            egui::FontId::proportional(12.),
                            color,
                        );
                    }
                }
            }
        }

        let text = if let Some([a, b]) = self.authoring.measure.segment(hover) {
            let delta = b - a;
            let a = self.authoring.board.screen(rect, a);
            let b = self.authoring.board.screen(rect, b);
            let color = ui.visuals().selection.stroke.color;
            painter.line_segment([a, b], Stroke::new(2., color));
            for point in [a, b] {
                painter.circle_filled(point, 4., color);
                painter.circle_stroke(point, 7., Stroke::new(1., color));
            }
            format!(
                "{:.1} mm  ·  ΔX {:+.1}  ΔY {:+.1} mm  ·  {:.1}°",
                delta.length() * 1000.,
                delta.x * 1000.,
                delta.y * 1000.,
                delta.y.atan2(delta.x).to_degrees()
            )
        } else {
            "Click the first point · then click the second point".into()
        };
        let galley = painter.layout_no_wrap(
            text,
            egui::FontId::proportional(14.),
            ui.visuals().text_color(),
        );
        let at = rect.left_top() + Vec2::splat(12.);
        painter.rect_filled(
            egui::Rect::from_min_size(at - Vec2::splat(6.), galley.size() + Vec2::splat(12.)),
            4.,
            ui.visuals().window_fill(),
        );
        painter.galley(at, galley, ui.visuals().text_color());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_clicks_lock_a_measurement_third_starts_another() {
        let mut tape = Tape::default();
        tape.click(DVec2::ZERO);
        assert_eq!(
            tape.segment(Some(DVec2::new(0.3, 0.4))).unwrap()[0],
            DVec2::ZERO
        );
        tape.click(DVec2::new(0.3, 0.4));
        let [a, b] = tape.segment(Some(DVec2::ONE)).unwrap();
        assert!((a.distance(b) - 0.5).abs() < 1e-12);
        tape.click(DVec2::ONE);
        assert!(tape.segment(None).is_none());
    }

    #[test]
    fn tape_tracks_unsaved_draft_state() {
        let mut tape = Tape::default();
        assert!(!tape.has_draft());
        tape.click(DVec2::ZERO);
        assert!(tape.has_draft());
        tape.click(DVec2::new(0.3, 0.4));
        assert!(tape.has_draft());
        tape.click(DVec2::ONE);
        assert!(!tape.has_draft());
    }

    #[test]
    fn named_station_resolves_back_to_the_boundary_fraction() {
        use marvelous_cloth::garment::{meshing::MeshingSettings, polygon::PolygonPattern};
        let polygon = PolygonPattern {
            points: vec![
                DVec2::ZERO,
                DVec2::new(0.4, 0.),
                DVec2::new(0.4, 0.3),
                DVec2::new(0., 0.3),
            ],
            edges: vec!["bottom".into(), "right".into(), "top".into(), "left".into()],
            subdivisions: 0,
            meshing: Some(MeshingSettings::default()),
            curves: Default::default(),
            holes: vec![],
            lines: vec![],
            target_lengths: Default::default(),
            seam_allowances: Default::default(),
            notches: Default::default(),
        };
        let station = station_for_point("panel", &polygon, DVec2::new(0.2, 0.)).unwrap();
        assert_eq!(station.boundary_id, "bottom");
        assert!((station.fraction - 0.5).abs() < 1e-12);
        let point = station_point("panel", &polygon, &station).unwrap();
        assert!(point.distance(DVec2::new(0.2, 0.)) < 1e-12);
    }
}
