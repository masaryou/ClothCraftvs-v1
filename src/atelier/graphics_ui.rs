//! Native Piece-inspector controls for material-space garment graphics.
//! Asset loading and 3D rendering remain separate; this module owns placement metadata only.
use super::*;
use marvelous_cloth::garment::graphics::{GraphicPlacement, GraphicSide};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GraphicDraft {
    pub asset_id: String,
    pub center_x_mm: f64,
    pub center_y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub rotation_degrees: f64,
    pub side: GraphicSide,
    pub selected: Option<String>,
}

impl Default for GraphicDraft {
    fn default() -> Self {
        Self {
            asset_id: "asset_1".into(),
            center_x_mm: 0.,
            center_y_mm: 0.,
            width_mm: 100.,
            height_mm: 100.,
            rotation_degrees: 0.,
            side: GraphicSide::Front,
            selected: None,
        }
    }
}

fn named(ui: &egui::Ui, response: &egui::Response, name: &str) {
    #[cfg(test)]
    super::authoring_smoke::widget_harness::named(ui, response, name);
    #[cfg(not(test))]
    let _ = (ui, response, name);
}

impl super::Atelier {
    pub(super) fn paint_graphic_overlays(&self, painter: &egui::Painter, rect: egui::Rect) {
        let color = egui::Color32::from_rgb(0x6A, 0x3D, 0xA3);
        for graphic in &self.project.template.graphics {
            let Some(panel_index) = self
                .project
                .template
                .panels
                .iter()
                .position(|panel| panel.id == graphic.panel_id)
            else {
                continue;
            };
            let panel = &self.project.template.panels[panel_index];
            let origin = marvelous_cloth::authoring::layout(&self.project.template, panel_index);
            let angle = panel.layout_rotation_degrees;
            let corners = [
                glam::DVec2::new(-0.5, -0.5),
                glam::DVec2::new(0.5, -0.5),
                glam::DVec2::new(0.5, 0.5),
                glam::DVec2::new(-0.5, 0.5),
            ]
            .map(|uv| {
                graphic
                    .uv_to_material(uv)
                    .map(|point| origin + marvelous_cloth::authoring::rotate_2d(point, angle))
            });
            if corners.iter().all(|corner| corner.is_ok()) {
                let points: Vec<_> = corners
                    .iter()
                    .map(|corner| self.authoring.board.screen(rect, *corner.as_ref().unwrap()))
                    .collect();
                for pair in points
                    .iter()
                    .enumerate()
                    .map(|(i, point)| (*point, points[(i + 1) % points.len()]))
                {
                    painter.line_segment([pair.0, pair.1], egui::Stroke::new(1.5, color));
                }
            }
        }
    }

    pub(super) fn graphics_properties(
        &mut self,
        ui: &mut egui::Ui,
        panel: &marvelous_cloth::garment::template::Panel,
    ) {
        ui.separator();
        ui.strong("Garment graphics");
        ui.small("Opaque asset IDs only; image loading and rendering are separate.");

        let mut remove = None;
        let graphics: Vec<_> = self
            .project
            .template
            .graphics
            .iter()
            .filter(|graphic| graphic.panel_id == panel.id)
            .cloned()
            .collect();
        for graphic in &graphics {
            ui.horizontal(|ui| {
                let row = ui.selectable_label(
                    self.authoring.graphic_draft.selected.as_deref() == Some(graphic.id.as_str()),
                    format!("{} · {}", graphic.asset_id, graphic.id),
                );
                named(ui, &row, &format!("graphics.row.{}", graphic.id));
                if row.clicked() {
                    self.authoring.graphic_draft.selected = Some(graphic.id.clone());
                    self.authoring.graphic_draft.asset_id = graphic.asset_id.clone();
                    self.authoring.graphic_draft.center_x_mm = graphic.center_m.x * 1000.;
                    self.authoring.graphic_draft.center_y_mm = graphic.center_m.y * 1000.;
                    self.authoring.graphic_draft.width_mm = graphic.size_m.x * 1000.;
                    self.authoring.graphic_draft.height_mm = graphic.size_m.y * 1000.;
                    self.authoring.graphic_draft.rotation_degrees = graphic.rotation_degrees;
                    self.authoring.graphic_draft.side = graphic.side;
                }
                let delete = ui.button("Delete");
                named(ui, &delete, &format!("graphics.delete.{}", graphic.id));
                if delete.clicked() {
                    remove = Some(graphic.id.clone());
                }
            });
        }

        let draft = &mut self.authoring.graphic_draft;
        ui.add(egui::TextEdit::singleline(&mut draft.asset_id).hint_text("Asset ID"));
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
            egui::DragValue::new(&mut draft.width_mm)
                .range(1.0..=100000.0)
                .speed(1.)
                .suffix(" mm W"),
        );
        ui.add(
            egui::DragValue::new(&mut draft.height_mm)
                .range(1.0..=100000.0)
                .speed(1.)
                .suffix(" mm H"),
        );
        ui.add(
            egui::DragValue::new(&mut draft.rotation_degrees)
                .range(-360.0..=360.0)
                .speed(1.)
                .suffix(" deg"),
        );
        ui.horizontal(|ui| {
            let front = ui.radio(draft.side == GraphicSide::Front, "Front");
            named(ui, &front, "graphics.side.front");
            if front.clicked() {
                draft.side = GraphicSide::Front;
            }
            let back = ui.radio(draft.side == GraphicSide::Back, "Back");
            named(ui, &back, "graphics.side.back");
            if back.clicked() {
                draft.side = GraphicSide::Back;
            }
        });

        let apply = ui.button("Apply placement");
        named(ui, &apply, "graphics.apply");
        if apply.clicked() {
            let draft_values = (
                draft.asset_id.clone(),
                draft.center_x_mm,
                draft.center_y_mm,
                draft.width_mm,
                draft.height_mm,
                draft.rotation_degrees,
                draft.side,
                draft.selected.clone(),
            );
            let mut number = 1usize;
            let id = if let Some(selected) = draft_values.7.clone() {
                selected
            } else {
                loop {
                    let candidate = format!("graphic_{number}");
                    number += 1;
                    if !self
                        .project
                        .template
                        .graphics
                        .iter()
                        .any(|graphic| graphic.id == candidate)
                    {
                        break candidate;
                    }
                }
            };
            let placement = GraphicPlacement {
                id,
                panel_id: panel.id.clone(),
                asset_id: draft_values.0.trim().to_string(),
                center_m: glam::DVec2::new(draft_values.1 / 1000., draft_values.2 / 1000.),
                size_m: glam::DVec2::new(draft_values.3 / 1000., draft_values.4 / 1000.),
                rotation_degrees: draft_values.5,
                side: draft_values.6,
            };
            let clear_selected =
                self.edit(marvelous_cloth::authoring::Command::SetGraphic { placement });
            if clear_selected {
                self.authoring.graphic_draft.selected = None;
            }
        }

        if let Some(id) = remove {
            let selected = self.authoring.graphic_draft.selected.clone();
            let cleared =
                self.edit(marvelous_cloth::authoring::Command::RemoveGraphic { id: id.clone() });
            if cleared && selected.as_deref() == Some(id.as_str()) {
                self.authoring.graphic_draft.selected = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphic_draft_defaults_are_explicit_and_valid() {
        let draft = GraphicDraft::default();
        assert_eq!(draft.asset_id, "asset_1");
        assert_eq!(draft.width_mm, 100.);
        assert_eq!(draft.height_mm, 100.);
        assert_eq!(draft.side, GraphicSide::Front);

        let placement = GraphicPlacement {
            id: "graphic_test".into(),
            panel_id: "front".into(),
            asset_id: draft.asset_id,
            center_m: glam::DVec2::new(draft.center_x_mm / 1000., draft.center_y_mm / 1000.),
            size_m: glam::DVec2::new(draft.width_mm / 1000., draft.height_mm / 1000.),
            rotation_degrees: draft.rotation_degrees,
            side: draft.side,
        };
        assert!(placement.validate().is_ok());
    }
}
