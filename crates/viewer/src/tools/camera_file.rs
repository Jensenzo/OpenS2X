//! `--camera <file>`: the camera of a look-comparison shot (TOOL-trace
//! TRACE-8), as the investigation reads it from the original: a flat JSON
//! object with `eye` and `dir` (the camera's position and its view
//! direction in the game frame of GFX-VIEW-1), and for reference `target`,
//! `zoom`, `yaw`, `tick`, `window` and the map's name. Only these fields
//! are read; any others are left alone.

/// The fields of a camera file this program uses.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CameraFile {
    pub(crate) eye: Option<[f32; 3]>,
    pub(crate) dir: Option<[f32; 3]>,
    pub(crate) target: Option<[f32; 3]>,
    pub(crate) zoom: Option<f32>,
    pub(crate) yaw: Option<f32>,
    pub(crate) tick: Option<u64>,
    pub(crate) window: Option<[u32; 2]>,
    pub(crate) map: Option<String>,
}

impl CameraFile {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let three = |key: &str| -> Result<Option<[f32; 3]>, String> {
            match numbers(text, key)? {
                None => Ok(None),
                Some(v) if v.len() == 3 => Ok(Some([v[0] as f32, v[1] as f32, v[2] as f32])),
                Some(v) => Err(format!("{key}: {} numbers, not 3", v.len())),
            }
        };
        let one = |key: &str| -> Result<Option<f64>, String> {
            match numbers(text, key)? {
                None => Ok(None),
                Some(v) if v.len() == 1 => Ok(Some(v[0])),
                Some(v) => Err(format!("{key}: {} numbers, not 1", v.len())),
            }
        };
        let window = match numbers(text, "window")? {
            None => None,
            Some(v) if v.len() == 2 => Some([v[0] as u32, v[1] as u32]),
            Some(v) => return Err(format!("window: {} numbers, not 2", v.len())),
        };
        let file = Self {
            eye: three("eye")?,
            dir: three("dir")?,
            target: three("target")?,
            zoom: one("zoom")?.map(|z| z as f32),
            yaw: one("yaw")?.map(|y| y as f32),
            tick: one("tick")?.map(|t| t as u64),
            window,
            map: string(text, "map"),
        };
        if file.eye.is_none() || file.dir.is_none() {
            return Err("the camera file needs eye and dir".into());
        }
        Ok(file)
    }
}

/// Where the value of `"key":` starts, if the key is there.
fn value_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let quoted = format!("\"{key}\"");
    let at = text.find(&quoted)? + quoted.len();
    let rest = text[at..].trim_start().strip_prefix(':')?;
    Some(rest.trim_start())
}

/// The number, or the numbers of the array, of `key`.
fn numbers(text: &str, key: &str) -> Result<Option<Vec<f64>>, String> {
    let Some(rest) = value_of(text, key) else {
        return Ok(None);
    };
    let (list, single) = match rest.strip_prefix('[') {
        Some(inner) => (
            &inner[..inner.find(']').ok_or(format!("{key}: no ]"))?],
            false,
        ),
        None => (
            &rest[..rest.find([',', '}', '\n']).unwrap_or(rest.len())],
            true,
        ),
    };
    let values: Result<Vec<f64>, _> = list
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::parse::<f64>)
        .collect();
    let values = values.map_err(|e| format!("{key}: {e}"))?;
    if single && values.len() != 1 {
        return Err(format!("{key}: not a number"));
    }
    Ok(Some(values))
}

/// The string of `key`.
fn string(text: &str, key: &str) -> Option<String> {
    let rest = value_of(text, key)?.strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: none (the look comparison's camera file, TOOL-trace TRACE-8)
    #[test]
    fn a_camera_file_is_read() {
        let text = r#"{
  "tick": 11000, "map": "MP_2P_Storm_Coast", "game": "weak",
  "window": [1024, 768],
  "eye": [120.5, 55.25, -3.0], "dir": [0.0, -0.8, 0.6],
  "target": [120.5, 5.0, 18.0],
  "zoom": 3, "yaw": 0.0, "fov_h_deg": 50.0,
  "view": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
}"#;
        let c = CameraFile::parse(text).unwrap();
        assert_eq!(c.eye, Some([120.5, 55.25, -3.0]));
        assert_eq!(c.dir, Some([0.0, -0.8, 0.6]));
        assert_eq!(c.target, Some([120.5, 5.0, 18.0]));
        assert_eq!(c.zoom, Some(3.0));
        assert_eq!(c.tick, Some(11000));
        assert_eq!(c.window, Some([1024, 768]));
        assert_eq!(c.map.as_deref(), Some("MP_2P_Storm_Coast"));
        assert!(CameraFile::parse(r#"{"eye": [1, 2, 3]}"#).is_err());
    }
}
