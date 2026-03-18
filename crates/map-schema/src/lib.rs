use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TownMap {
    pub grid: Vec<Vec<CellType>>,
    pub props: Vec<PropSpawn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellType {
    Grass,
    RoadNs,
    RoadEw,
    RoadIntersection,
    BuildingZone,
    Parking,
    Park,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropSpawn {
    pub model: String,
    pub position: [f32; 3],
    pub yaw: f32,
}

#[derive(Debug)]
pub enum MapError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Validation(Vec<String>),
}

impl Display for MapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "Failed to read map {}: {source}", path.display()),
            Self::Parse { path, source } => {
                write!(f, "Failed to parse map {} as JSON: {source}", path.display())
            }
            Self::Validation(errors) => write!(f, "Invalid map: {}", errors.join("; ")),
        }
    }
}

impl Error for MapError {}

pub fn load_map(path: &Path) -> Result<TownMap, MapError> {
    let data = fs::read_to_string(path).map_err(|source| MapError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let map: TownMap = serde_json::from_str(&data).map_err(|source| MapError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    validate(&map).map_err(MapError::Validation)?;
    Ok(map)
}

pub fn save_map(path: &Path, map: &TownMap) -> Result<(), MapError> {
    validate(map).map_err(MapError::Validation)?;

    let data = serde_json::to_string_pretty(map).expect("serializing TownMap should never fail");
    fs::write(path, data).map_err(|source| MapError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn validate(map: &TownMap) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if map.grid.is_empty() {
        errors.push("grid must contain at least one row".to_owned());
    }

    let mut expected_len: Option<usize> = None;
    for (row_idx, row) in map.grid.iter().enumerate() {
        if row.is_empty() {
            errors.push(format!("grid row {row_idx} must not be empty"));
            continue;
        }
        if let Some(expected) = expected_len {
            if row.len() != expected {
                errors.push(format!(
                    "grid row {row_idx} has length {}, expected {expected}",
                    row.len()
                ));
            }
        } else {
            expected_len = Some(row.len());
        }
    }

    for (idx, prop) in map.props.iter().enumerate() {
        if prop.model.trim().is_empty() {
            errors.push(format!("props[{idx}].model must not be empty"));
        }
        if !prop.position.iter().all(|value| value.is_finite()) {
            errors.push(format!("props[{idx}].position must contain finite values"));
        }
        if !prop.yaw.is_finite() {
            errors.push(format!("props[{idx}].yaw must be finite"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub const TILE_PALETTE: &[CellType] = &[
    CellType::Grass,
    CellType::RoadNs,
    CellType::RoadEw,
    CellType::RoadIntersection,
    CellType::BuildingZone,
    CellType::Parking,
    CellType::Park,
];

pub const PROP_PALETTE: &[&str] = &[
    "detail-light-single.glb",
    "detail-bench.glb",
    "detail-dumpster-closed.glb",
    "detail-dumpster-open.glb",
    "tree-large.glb",
    "tree-pine-large.glb",
    "truck-green.glb",
];

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn valid_map_round_trip_and_validation() {
        let map = TownMap {
            grid: vec![vec![CellType::Grass, CellType::RoadNs]],
            props: vec![PropSpawn {
                model: "detail-bench.glb".to_owned(),
                position: [1.0, 0.0, 2.0],
                yaw: 0.0,
            }],
        };

        validate(&map).expect("map should validate");

        let serialized = serde_json::to_string(&map).expect("serialize");
        let deserialized: TownMap = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.grid.len(), 1);
        assert_eq!(deserialized.grid[0].len(), 2);
        assert_eq!(deserialized.props.len(), 1);
    }

    #[test]
    fn invalid_map_rejected() {
        let map = TownMap {
            grid: vec![vec![CellType::Grass], vec![]],
            props: vec![PropSpawn {
                model: "".to_owned(),
                position: [1.0, f32::NAN, 2.0],
                yaw: f32::INFINITY,
            }],
        };

        let errors = validate(&map).expect_err("map should be invalid");
        assert!(!errors.is_empty());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json = r#"
        {
          "grid": [["Grass"]],
          "props": [],
          "extra": 1
        }
        "#;

        let parsed = serde_json::from_str::<TownMap>(json);
        assert!(parsed.is_err());
    }

    #[test]
    fn load_and_save_map_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("retro_urban_map_schema_test.json");

        let map = TownMap {
            grid: vec![vec![CellType::Grass, CellType::Parking]],
            props: vec![],
        };

        save_map(&path, &map).expect("save should succeed");
        let loaded = load_map(&path).expect("load should succeed");
        fs::remove_file(&path).ok();

        assert_eq!(loaded.grid[0].len(), 2);
    }
}
