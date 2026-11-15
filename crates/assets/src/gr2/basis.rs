//! GFX-VIEW-1.2, 1.3 (`specs/GFX-world-view.md`): the basis conversion
//! that turns a Granny file's own axes and units (its art tool info) into
//! the game's frame.

use super::ArtToolInfo;

/// A linear map and a shift, `p -> linear p + translation`, rows of the
/// matrix in `linear`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis {
    pub linear: [[f32; 3]; 3],
    pub translation: [f32; 3],
}

pub fn mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

fn mat_mul(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    [0, 1, 2].map(|r| [0, 1, 2].map(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

fn determinant(m: &[[f32; 3]; 3]) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn inverse(m: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let det = determinant(m);
    if det.abs() < 1e-9 {
        return None;
    }
    let c =
        |r0: usize, c0: usize, r1: usize, c1: usize| m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0];
    // The adjugate: transposed cofactors.
    let adj = [
        [c(1, 1, 2, 2), -c(0, 1, 2, 2), c(0, 1, 1, 2)],
        [-c(1, 0, 2, 2), c(0, 0, 2, 2), -c(0, 0, 1, 2)],
        [c(1, 0, 2, 1), -c(0, 0, 2, 1), c(0, 0, 1, 1)],
    ];
    Some(adj.map(|row| row.map(|x| x / det)))
}

fn transpose(m: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    [0, 1, 2].map(|r| [0, 1, 2].map(|c| m[c][r]))
}

impl Basis {
    pub fn apply(&self, p: [f32; 3]) -> [f32; 3] {
        let v = mul(&self.linear, p);
        [0, 1, 2].map(|i| v[i] + self.translation[i])
    }

    /// The map normals take: the inverse transpose of the linear part.
    pub fn normal_map(&self) -> [[f32; 3]; 3] {
        inverse(&self.linear).map_or(self.linear, |inv| transpose(&inv))
    }

    /// The linear part mirrors (its determinant is negative).
    pub fn mirrors(&self) -> bool {
        determinant(&self.linear) < 0.0
    }

    /// The same conversion followed by a flip of z: into a right-handed
    /// frame with y up (Bevy's, glTF's).
    pub fn flip_z(self) -> Self {
        let flip = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
        let [x, y, z] = self.translation;
        Self {
            linear: mat_mul(&flip, &self.linear),
            translation: [x, y, -z],
        }
    }
}

/// GFX-VIEW-1.2: the game's target basis: origin 0, right +z, up +y, back
/// +x (as matrix columns).
const TARGET_AXES: [[f32; 3]; 3] = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];

/// GFX-VIEW-1.2, 1.3: the conversion from a file's own basis (its art tool
/// info: origin, right, up, back, units per metre) to the game frame at
/// `units_per_metre`: a vector given in the file's axes keeps its
/// components in the target axes, scaled by the ratio of the units per
/// metre. A file without art tool info is taken as right +x, up +y, back
/// +z at 1 unit per metre.
pub fn basis_conversion(art: Option<&ArtToolInfo>, units_per_metre: f32) -> Basis {
    let (origin, right, up, back, file_units) = match art {
        Some(a) => (a.origin, a.right, a.up, a.back, a.units_per_meter),
        None => (
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            1.0,
        ),
    };
    // File axes as columns.
    let file = [0, 1, 2].map(|r| [right[r], up[r], back[r]]);
    let target = [0, 1, 2].map(|r| [TARGET_AXES[0][r], TARGET_AXES[1][r], TARGET_AXES[2][r]]);
    let scale = if file_units > 0.0 {
        units_per_metre / file_units
    } else {
        units_per_metre
    };
    let inverse = inverse(&file).unwrap_or([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    let linear = mat_mul(&target, &inverse).map(|row| row.map(|x| x * scale));
    let shifted = mul(&linear, origin);
    Basis {
        linear,
        translation: shifted.map(|x| -x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5)
    }

    // GFX-VIEW T2: right +x, up +y, back +z at 1 unit per metre, settler
    // scale 1.2.
    // spec: GFX-VIEW-1.2
    #[test]
    fn basis_conversion_of_t2() {
        let art = ArtToolInfo {
            tool: String::new(),
            units_per_meter: 1.0,
            origin: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            back: [0.0, 0.0, 1.0],
        };
        let basis = basis_conversion(Some(&art), 1.2);
        assert!(close(basis.apply([1.0, 0.0, 0.0]), [0.0, 0.0, 1.2]));
        assert!(close(basis.apply([0.0, 0.0, 1.0]), [1.2, 0.0, 0.0]));
        // A swap of two axes mirrors; with a z flip it does not.
        assert!(basis.mirrors());
        assert!(!basis.flip_z().mirrors());
    }

    // The game's files: 3D Studio MAX, right +x, up +z, back -y. Up becomes
    // the game's +y, right its +z, back its +x.
    // spec: GFX-VIEW-1.2
    #[test]
    fn basis_conversion_of_max_files() {
        let art = ArtToolInfo {
            tool: "3D Studio MAX".into(),
            units_per_meter: 1.0,
            origin: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 0.0, 1.0],
            back: [0.0, -1.0, 0.0],
        };
        let basis = basis_conversion(Some(&art), 1.0);
        assert!(close(basis.apply([0.0, 0.0, 1.0]), [0.0, 1.0, 0.0]));
        assert!(close(basis.apply([1.0, 0.0, 0.0]), [0.0, 0.0, 1.0]));
        assert!(close(basis.apply([0.0, -1.0, 0.0]), [1.0, 0.0, 0.0]));
        // Max is right-handed, the game left-handed: a mirror, undone by
        // a z flip.
        assert!(basis.mirrors() && !basis.flip_z().mirrors());
        // A file at 39.37 units per metre drawn at 1.5 shrinks by 1.5/39.37.
        let inches = ArtToolInfo {
            units_per_meter: 39.37,
            ..art
        };
        let b = basis_conversion(Some(&inches), 1.5);
        assert!(close(b.apply([0.0, 0.0, 39.37]), [0.0, 1.5, 0.0]));
        // Normals keep their direction under a uniform scale.
        let n = mul(&b.normal_map(), [0.0, 0.0, 1.0]);
        assert!(n[1] > 0.0 && n[0].abs() < 1e-6 && n[2].abs() < 1e-6);
    }
}
