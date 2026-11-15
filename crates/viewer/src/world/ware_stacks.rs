//! Wares lying at a flag (`specs/GFX-objects.md` GFX-OBJ-5): up to six
//! stacks around the flag, one per good, up to six wares each, laid out by
//! the good's stack layout.

/// GFX-OBJ-5.1: stacks per flag and wares per stack.
pub const STACKS: usize = 6;
pub const PER_STACK: usize = 6;

/// A flag's stacks: the good of each and its wares in order, kept from
/// tick to tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlagStacks {
    pub stacks: [Option<(u32, Vec<usize>)>; STACKS],
}

impl FlagStacks {
    /// GFX-OBJ-5.1, 5.2: wares that left are taken out (the later ones of
    /// their stack move down a slot, an empty stack is freed); a new ware
    /// joins the stack of its good, else the first free stack. A ware with
    /// no room is not shown.
    pub fn update(&mut self, wares: &[(usize, u32)]) {
        for slot in self.stacks.iter_mut() {
            if let Some((_, list)) = slot {
                list.retain(|w| wares.iter().any(|(id, _)| id == w));
                if list.is_empty() {
                    *slot = None;
                }
            }
        }
        for &(id, good) in wares {
            if self
                .stacks
                .iter()
                .flatten()
                .any(|(_, list)| list.contains(&id))
            {
                continue;
            }
            let own = self.stacks.iter().position(|s| {
                s.as_ref()
                    .is_some_and(|(g, list)| *g == good && list.len() < PER_STACK)
            });
            let at = own.or_else(|| self.stacks.iter().position(Option::is_none));
            let Some(at) = at else { continue };
            self.stacks[at]
                .get_or_insert_with(|| (good, Vec::new()))
                .1
                .push(id);
        }
    }

    /// Each shown ware with its stack and its slot in the stack.
    pub fn slots(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        self.stacks.iter().enumerate().flat_map(|(i, s)| {
            s.iter()
                .flat_map(move |(_, list)| list.iter().enumerate().map(move |(k, &w)| (w, i, k)))
        })
    }
}

/// GFX-OBJ-5.1, 5.2: where the k-th ware of stack i lies relative to the
/// flag (game x, y, z) and its heading: the stack stands at angle i x 60
/// degrees, radius = the ware's bounding-box depth + `distance`; the ware
/// at the layout offset times the box's extent, turned by the angle.
pub fn ware_offset(
    stack: usize,
    k: usize,
    offsets: &[[f32; 3]; 6],
    extent: [f32; 3],
    distance: f32,
) -> ([f32; 3], f32) {
    let a = (stack as f32 * 60.0).to_radians();
    let r = extent[2] + distance;
    let base = [a.sin() * r, 0.0, a.cos() * r];
    let o = offsets[k.min(5)];
    let local = [o[0] * extent[0], o[1] * extent[1], o[2] * extent[2]];
    // A turn by `a` about +y.
    let turned = [
        local[0] * a.cos() + local[2] * a.sin(),
        local[1],
        -local[0] * a.sin() + local[2] * a.cos(),
    ];
    ([base[0] + turned[0], turned[1], base[2] + turned[2]], a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: [[f32; 3]; 6] = [
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [-0.5, 1.0, 0.0],
        [0.5, 1.0, 0.0],
        [0.0, 2.0, 0.0],
    ];

    /// GFX-OBJ T5: 2 Wood and 1 Stone: the Wood stack at angle 0 with the
    /// wares at (-1, 0, 0) and (0, 0, 0) x the box; the Stone stack at 60.
    // spec: GFX-OBJ-5.1, GFX-OBJ-5.2 (T5)
    #[test]
    fn t5_stacks() {
        let (wood, stone) = (1, 2);
        let mut f = FlagStacks::default();
        f.update(&[(10, wood), (11, wood), (12, stone)]);
        let slots: Vec<_> = f.slots().collect();
        assert_eq!(slots, [(10, 0, 0), (11, 0, 1), (12, 1, 0)]);
        let extent = [2.0, 0.5, 0.4];
        let (p, heading) = ware_offset(0, 0, &A, extent, 0.5);
        assert!((p[0] + 2.0).abs() < 1e-6 && (p[2] - 0.9).abs() < 1e-6);
        assert_eq!(heading, 0.0);
        let (p, _) = ware_offset(0, 1, &A, extent, 0.5);
        assert!(p[0].abs() < 1e-6 && (p[2] - 0.9).abs() < 1e-6);
        let (p, heading) = ware_offset(1, 0, &A, extent, 0.5);
        assert!((heading - 60f32.to_radians()).abs() < 1e-6);
        let r = 0.9;
        let a = 60f32.to_radians();
        assert!((p[0] - (a.sin() * r - 2.0 * a.cos())).abs() < 1e-5);
    }

    /// A ware leaving moves the later ones down; an empty stack is freed
    /// and the next good takes the first free one.
    // spec: GFX-OBJ-5.1
    #[test]
    fn leaving_and_freeing() {
        let mut f = FlagStacks::default();
        f.update(&[(1, 5), (2, 5), (3, 7)]);
        f.update(&[(2, 5), (3, 7)]);
        assert_eq!(f.stacks[0], Some((5, vec![2])));
        f.update(&[(3, 7)]);
        assert_eq!(f.stacks[0], None);
        f.update(&[(3, 7), (4, 9)]);
        assert_eq!(f.stacks[0], Some((9, vec![4])));
        assert_eq!(f.stacks[1], Some((7, vec![3])));
    }
}
