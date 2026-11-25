//! Native Piece-inspector controls for nonphysical garment trims.
use super::*;
use marvelous_cloth::garment::trims::{Trim, TrimKind};

#[derive(Default)]
pub(super) struct TrimDraft {
    pub kind: TrimKindDraft,
    pub center_x_mm: f64,
    pub center_y_mm: f64,
    pub size_mm: f64,
    pub length_mm: f64,
    pub selected: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) enum TrimKindDraft {
    #[default]
    Button,
    Buttonhole,
    Zipper,
    Topstitch,
}

fn named(ui: &egui::Ui, response: &egui::Response, name: &str) {
    #[cfg(test)]
    super::authoring_smoke::widget_harness::named(ui, response, name);
    #[cfg(not(test))]
    let _ = (ui, response, name);
}

impl Atelier {
    pub(super) fn trim_world_overlays(&self) -> Vec<[glam::Vec3; 2]> {
        let mut overlays = Vec::new();
        for trim in &self.project.template.trims {
            let points: Vec<glam::DVec2> = match &trim.kind {
                TrimKind::Button { center_m, .. } => vec![*center_m],
                TrimKind::Buttonhole { start_m, end_m, .. } => vec![*start_m, *end_m],
                TrimKind::Zipper { path_m, .. } | TrimKind::Topstitch { path_m, .. } => {
                    path_m.clone()
                }
            };
            for pair in points.windows(2) {
                let left = self.scene.garment.embed_point(&trim.panel_id, pair[0]);
                let right = self.scene.garment.embed_point(&trim.panel_id, pair[1]);
                if let (Ok(left), Ok(right)) = (left, right) {
                    if let (Ok(a), Ok(b)) = (
                        left.evaluate(&self.scene.state.x),
                        right.evaluate(&self.scene.state.x),
                    ) {
                        overlays.push([a.as_vec3(), b.as_vec3()]);
                    }
                }
            }
            if let TrimKind::Button { center_m, .. } = &trim.kind {
                if let Ok(point) = self.scene.garment.embed_point(&trim.panel_id, *center_m) {
                    if let Ok(center) = point.evaluate(&self.scene.state.x) {
                        let radius = 0.0015;
                        overlays.push([
                            center.as_vec3() - glam::Vec3::X * radius,
                            center.as_vec3() + glam::Vec3::X * radius,
                        ]);
                    }
                }
            }
        }
        overlays
    }

    pub(super) fn paint_trim_overlays(&self, painter: &egui::Painter, rect: egui::Rect) {
        let color = egui::Color32::from_rgb(0xC4, 0x73, 0x00);
        for trim in &self.project.template.trims {
            let Some(panel_index) = self
                .project
                .template
                .panels
                .iter()
                .position(|panel| panel.id == trim.panel_id)
            else {
                continue;
            };
            let panel = &self.project.template.panels[panel_index];
            let origin = marvelous_cloth::authoring::layout(&self.project.template, panel_index);
            let points = match &trim.kind {
                TrimKind::Button {
                    center_m,
                    diameter_m,
                } => {
                    let center = origin
                        + marvelous_cloth::authoring::rotate_2d(
                            *center_m,
                            panel.layout_rotation_degrees,
                        );
                    let screen = self.authoring.board.screen(rect, center);
                    let radius =
                        (*diameter_m * 1000. * f64::from(self.authoring.board.scale) * 0.5) as f32;
                    painter.circle_stroke(screen, radius, egui::Stroke::new(1.5, color));
                    continue;
                }
                TrimKind::Buttonhole {
                    start_m,
                    end_m,
                    width_m: _,
                } => {
                    vec![*start_m, *end_m]
                }
                TrimKind::Zipper { path_m, .. } => path_m.clone(),
                TrimKind::Topstitch { path_m, .. } => path_m.clone(),
            };
            if let [a, b, ..] = points.as_slice() {
                let screen_points: Vec<_> = points
                    .iter()
                    .take(if matches!(trim.kind, TrimKind::Buttonhole { .. }) {
                        2
                    } else {
                        points.len()
                    })
                    .map(|point| {
                        let board = origin
                            + marvelous_cloth::authoring::rotate_2d(
                                *point,
                                panel.layout_rotation_degrees,
                            );
                        self.authoring.board.screen(rect, board)
                    })
                    .collect();
                for pair in screen_points.windows(2) {
                    painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, color));
                }
                if matches!(trim.kind, TrimKind::Buttonhole { .. }) {
                    let start = self.authoring.board.screen(
                        rect,
                        origin
                            + marvelous_cloth::authoring::rotate_2d(
                                *a,
                                panel.layout_rotation_degrees,
                            ),
                    );
                    let end = self.authoring.board.screen(
                        rect,
                        origin
                            + marvelous_cloth::authoring::rotate_2d(
                                *b,
                                panel.layout_rotation_degrees,
                            ),
                    );
                    painter.line_segment([start, end], egui::Stroke::new(2.5, color));
                }
            }
        }
    }

    pub(super) fn trim_properties(
        &mut self,
        ui: &mut egui::Ui,
        panel: &marvelous_cloth::garment::template::Panel,
    ) {
        ui.separator();
        ui.strong("Trims and construction");
        ui.small("Visual/manufacturing metadata; no physical fasteners or solver behavior.");

        let mut remove = None;
        let trims: Vec<_> = self
            .project
            .template
            .trims
            .iter()
            .filter(|trim| trim.panel_id == panel.id)
            .cloned()
            .collect();
        for trim in &trims {
            ui.horizontal(|ui| {
                let label = match &trim.kind {
                    TrimKind::Button { .. } => "Button",
                    TrimKind::Buttonhole { .. } => "Buttonhole",
                    TrimKind::Zipper { .. } => "Zipper",
                    TrimKind::Topstitch { .. } => "Topstitch",
                };
                let selected =
                    self.authoring.trim_draft.selected.as_deref() == Some(trim.id.as_str());
                let row = ui.selectable_label(selected, format!("{label}: {}", trim.id));
                named(ui, &row, &format!("trim.row.{}", trim.id));
                if row.clicked() {
                    self.authoring.trim_draft.selected = Some(trim.id.clone());
                }
                let delete = ui.button("Delete");
                named(ui, &delete, &format!("trim.delete.{}", trim.id));
                if delete.clicked() {
                    remove = Some(trim.id.clone());
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            let mut kind = self.authoring.trim_draft.kind;
            let button = ui.radio(kind == TrimKindDraft::Button, "Button");
            named(ui, &button, "trim.kind.button");
            if button.clicked() {
                kind = TrimKindDraft::Button;
            }
            let buttonhole = ui.radio(kind == TrimKindDraft::Buttonhole, "Buttonhole");
            named(ui, &buttonhole, "trim.kind.buttonhole");
            if buttonhole.clicked() {
                kind = TrimKindDraft::Buttonhole;
            }
            let zipper = ui.radio(kind == TrimKindDraft::Zipper, "Zipper");
            named(ui, &zipper, "trim.kind.zipper");
            if zipper.clicked() {
                kind = TrimKindDraft::Zipper;
            }
            let topstitch = ui.radio(kind == TrimKindDraft::Topstitch, "Topstitch");
            named(ui, &topstitch, "trim.kind.topstitch");
            if topstitch.clicked() {
                kind = TrimKindDraft::Topstitch;
            }
            self.authoring.trim_draft.kind = kind;
        });

        let draft = &mut self.authoring.trim_draft;
        ui.add(
            egui::DragValue::new(&mut draft.center_x_mm)
                .speed(1.)
                .suffix(" mm X"),
        );
        ui.add(
            egui::DragValue::new(&mut draft.center_y_mm)
                .speed(1.)
                .suffix(" mm Y"),
        );
        ui.add(
            egui::DragValue::new(&mut draft.size_mm)
                .range(1.0..=1000.0)
                .speed(1.)
                .suffix(" mm size"),
        );
        ui.add(
            egui::DragValue::new(&mut draft.length_mm)
                .range(1.0..=1000.0)
                .speed(1.)
                .suffix(" mm length"),
        );

        let add = ui.button("Add trim");
        named(ui, &add, "trim.add");
        if add.clicked() {
            let center = glam::DVec2::new(draft.center_x_mm / 1000., draft.center_y_mm / 1000.);
            let size = draft.size_mm.max(1.) / 1000.;
            let length = draft.length_mm.max(1.) / 1000.;
            let kind = match draft.kind {
                TrimKindDraft::Button => TrimKind::Button {
                    center_m: center,
                    diameter_m: size,
                },
                TrimKindDraft::Buttonhole => TrimKind::Buttonhole {
                    start_m: center,
                    end_m: center + glam::DVec2::new(length, 0.),
                    width_m: size.min(length),
                },
                TrimKindDraft::Zipper => TrimKind::Zipper {
                    path_m: vec![center, center + glam::DVec2::new(length, 0.)],
                    width_m: size,
                },
                TrimKindDraft::Topstitch => TrimKind::Topstitch {
                    path_m: vec![center, center + glam::DVec2::new(length, 0.)],
                    rows: 2,
                    spacing_m: size,
                },
            };
            let mut number = 1usize;
            let id = loop {
                let candidate = format!("trim_{number}");
                number += 1;
                if !self
                    .project
                    .template
                    .trims
                    .iter()
                    .any(|trim| trim.id == candidate)
                {
                    break candidate;
                }
            };
            let trim = Trim {
                id,
                panel_id: panel.id.clone(),
                kind,
            };
            let command = marvelous_cloth::authoring::Command::SetTrim { trim };
            if self.edit(command) {
                self.authoring.trim_draft.selected = None;
            }
        }

        if let Some(id) = remove
            && self.edit(marvelous_cloth::authoring::Command::RemoveTrim { id: id.clone() })
            && self.authoring.trim_draft.selected.as_deref() == Some(id.as_str())
        {
            self.authoring.trim_draft.selected = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_draft_defaults_and_kinds_are_safe() {
        let draft = TrimDraft::default();
        assert_eq!(draft.kind, TrimKindDraft::Button);
        assert_eq!(draft.size_mm, 0.0);
        assert_eq!(draft.length_mm, 0.0);

        let center = glam::DVec2::new(0.1, 0.2);
        let size = 0.018;
        let length = 0.025;
        let kinds = [
            TrimKind::Button {
                center_m: center,
                diameter_m: size,
            },
            TrimKind::Buttonhole {
                start_m: center,
                end_m: center + glam::DVec2::new(length, 0.),
                width_m: size,
            },
            TrimKind::Zipper {
                path_m: vec![center, center + glam::DVec2::new(length, 0.)],
                width_m: size,
            },
            TrimKind::Topstitch {
                path_m: vec![center, center + glam::DVec2::new(length, 0.)],
                rows: 2,
                spacing_m: size,
            },
        ];
        for kind in kinds {
            let trim = Trim {
                id: "trim_test".into(),
                panel_id: "front".into(),
                kind,
            };
            assert!(trim.validate().is_ok());
        }
    }
}
