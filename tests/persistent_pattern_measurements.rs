use marvelous_cloth::{
    authoring::{self, Command, History},
    garment::{
        measurements::{MeasurementStation, PatternMeasurement},
        presets::{self, GarmentKind},
    },
};

fn fixture() -> marvelous_cloth::garment::GarmentTemplate {
    presets::basic(GarmentKind::Tshirt, 0)
}

fn measurement(id: &str, label: &str) -> PatternMeasurement {
    PatternMeasurement {
        id: id.into(),
        label: label.into(),
        from: MeasurementStation {
            panel_id: "front".into(),
            boundary_id: "hem".into(),
            fraction: 0.0,
        },
        to: MeasurementStation {
            panel_id: "front".into(),
            boundary_id: "hem".into(),
            fraction: 1.0,
        },
    }
}

#[test]
fn create_rename_delete_round_trip_and_history() {
    let source = fixture();
    let created = authoring::apply(
        &source,
        Command::SetPatternMeasurement {
            measurement: measurement("hem", "Hem width"),
        },
    )
    .unwrap();
    assert!(!created.reset_simulation);
    assert_eq!(created.template.measurements.len(), 1);

    let json = serde_json::to_string(&created.template).unwrap();
    let loaded: marvelous_cloth::garment::GarmentTemplate = serde_json::from_str(&json).unwrap();
    loaded.compile().unwrap();
    assert_eq!(loaded.measurements, created.template.measurements);

    let mut history = History::default();
    history.record(source.clone());
    let renamed_value = PatternMeasurement {
        id: "hem".into(),
        label: "Lower hem".into(),
        from: measurement("hem", "Hem width").from,
        to: measurement("hem", "Hem width").to,
    };
    let renamed = authoring::apply(
        &created.template,
        Command::SetPatternMeasurement {
            measurement: renamed_value,
        },
    )
    .unwrap();
    assert!(!renamed.reset_simulation);

    let deleted = authoring::apply(
        &renamed.template,
        Command::RemovePatternMeasurement { id: "hem".into() },
    )
    .unwrap();
    assert!(!deleted.reset_simulation);
    assert!(deleted.template.measurements.is_empty());
}

#[test]
fn invalid_reference_is_atomic() {
    let source = fixture();
    let before = serde_json::to_value(&source).unwrap();

    let mut invalid = measurement("bad", "Bad measurement");
    invalid.from.panel_id = "missing".into();
    assert!(
        authoring::apply(
            &source,
            Command::SetPatternMeasurement {
                measurement: invalid,
            },
        )
        .is_err()
    );

    assert_eq!(serde_json::to_value(&source).unwrap(), before);
}

#[test]
fn split_duplicate_and_mirror_migrate_measurements() {
    let source = fixture();
    let mut station = measurement("hem", "Hem width");
    station.from.boundary_id = "hem".into();
    station.to.boundary_id = "hem".into();
    station.from.fraction = 0.05;
    station.to.fraction = 0.75;

    let created = authoring::apply(
        &source,
        Command::SetPatternMeasurement {
            measurement: station,
        },
    )
    .unwrap()
    .template;

    let split = authoring::apply(
        &created,
        Command::SplitBoundary {
            panel: "front".into(),
            edge: "hem".into(),
            at_m: 0.1,
        },
    )
    .unwrap()
    .template;
    let measurement = &split.measurements[0];
    assert_eq!(measurement.from.boundary_id, "hem");
    assert_eq!(measurement.to.boundary_id, "hem_2");

    let duplicate = authoring::apply(
        &created,
        Command::Duplicate {
            panel: "front".into(),
        },
    )
    .unwrap()
    .template;
    assert_eq!(duplicate.measurements.len(), 2);
    assert!(
        duplicate
            .measurements
            .iter()
            .any(|item| item.from.panel_id != "front")
    );

    let mirrored = authoring::apply(
        &created,
        Command::Mirror {
            panel: "front".into(),
            axis: marvelous_cloth::garment::shapes::MirrorAxis::X,
            copy_side: Some(1),
        },
    )
    .unwrap()
    .template;
    assert_eq!(mirrored.measurements.len(), 2);
    let copied = mirrored
        .measurements
        .iter()
        .find(|item| item.from.panel_id != "front")
        .unwrap();
    assert!((copied.from.fraction - 0.95).abs() < 1e-12);
    assert!((copied.to.fraction - 0.25).abs() < 1e-12);
}

#[test]
fn unchanged_and_duplicate_measurements_are_rejected() {
    let source = fixture();
    let created = authoring::apply(
        &source,
        Command::SetPatternMeasurement {
            measurement: measurement("hem", "Hem width"),
        },
    )
    .unwrap()
    .template;

    assert!(
        authoring::apply(
            &created,
            Command::SetPatternMeasurement {
                measurement: measurement("hem", "Hem width"),
            },
        )
        .is_err()
    );

    let duplicate_id = measurement("hem", "Second hem");
    assert!(
        authoring::apply(
            &created,
            Command::SetPatternMeasurement {
                measurement: duplicate_id,
            },
        )
        .is_ok()
    );
}
