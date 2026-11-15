//! GFX-GR2-7.5, 7.6: evaluation of the animation curves of
//! `specs/GFX-granny-format.md`, plain math on the decoded [`Curve`]s.

use super::Curve;

/// GFX-GR2-7.5: a curve's value at `t` (animation seconds, within
/// [0, last knot]), `rest` for an empty curve, the control for a one-knot
/// curve. The knot index k = max(1, knots strictly before t); the window
/// of 2d knots and controls k - d .. k + d - 1, clamped at the ends, or
/// for a clip that can still wrap (GFX-GR2-7.6) taken from the
/// neighbouring period; then the degree-0, -1 or -2 B-spline blend. An
/// orientation's controls copied into the window by clamping or wrapping
/// are negated when they point away from the window control before them,
/// and the blend is renormalised to first order, (3 - |q|^2) / 2.
pub fn sample<const N: usize>(curve: &Curve, t: f32, rest: [f32; N], looping: bool) -> [f32; N] {
    let n = (curve.controls.len() / N).min(curve.knots.len());
    if n == 0 {
        return rest;
    }
    let control = |i: usize| -> [f32; N] { std::array::from_fn(|c| curve.controls[i * N + c]) };
    if n == 1 {
        return control(0);
    }
    let knots = &curve.knots[..n];
    let last = n - 1;
    let d = curve.degree.min(2) as i64;
    let k = (knots.iter().filter(|&&x| x < t).count() as i64).max(1);
    // Window index i (in k - d .. k + d - 1): its knot, its control and
    // whether it was copied (clamped or wrapped) into the window.
    let entry = |i: i64| -> (f32, [f32; N], bool) {
        let n1 = last as i64;
        if looping && i < 0 {
            let j = (i + n1).max(0) as usize;
            (knots[j] - knots[last], control(j), true)
        } else if looping && i >= n1 {
            let j = ((i - n1) as usize).min(last);
            (knots[last] + knots[j], control(j), i > n1)
        } else if i < 0 {
            (knots[0], control(0), true)
        } else if i > n1 {
            (knots[last], control(last), true)
        } else {
            (knots[i as usize], control(i as usize), false)
        }
    };
    let first = k - d;
    let mut window: Vec<(f32, [f32; N])> = Vec::with_capacity(2 * d as usize);
    for i in first..k + d {
        let (knot, mut value, copied) = entry(i);
        if N == 4 && copied {
            let near = window
                .last()
                .map(|w| w.1)
                .or_else(|| (i + 1 < k + d).then(|| entry(i + 1).1));
            if let Some(near) = near {
                let dot: f32 = (0..N).map(|c| value[c] * near[c]).sum();
                if dot < 0.0 {
                    value = value.map(|v| -v);
                }
            }
        }
        window.push((knot, value));
    }
    // Knot and control of index i (relative to the window's start).
    let at = |i: i64| window[(i - first) as usize];
    let ratio = |num: f32, den: f32| if den.abs() > 0.0 { num / den } else { 0.0 };
    let mut value: [f32; N] = match d {
        0 => entry(k).1,
        1 => {
            let (t1, p0) = at(k - 1);
            let (t2, p1) = at(k);
            let u = ratio(t - t1, t2 - t1);
            std::array::from_fn(|c| u * p1[c] + (1.0 - u) * p0[c])
        }
        _ => {
            let (t0, pm2) = at(k - 2);
            let (t1, pm1) = at(k - 1);
            let (t2, pk) = at(k);
            let (t3, _) = at(k + 1);
            let a = ratio(t - t1, t2 - t1);
            let b = ratio(t - t0, t2 - t0);
            let c = a + b - a * b;
            let e = a * ratio(t - t1, t3 - t1);
            std::array::from_fn(|i| e * pk[i] + (c - e) * pm1[i] + (1.0 - c) * pm2[i])
        }
    };
    if N == 4 {
        let norm2: f32 = value.iter().map(|v| v * v).sum();
        let scale = (3.0 - norm2) / 2.0;
        value = value.map(|v| v * scale);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(degree: u32, knots: &[f32], controls: &[f32]) -> Curve {
        Curve {
            degree,
            knots: knots.to_vec(),
            controls: controls.to_vec(),
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    // spec: GFX-GR2-7.5
    #[test]
    fn empty_curve_is_the_rest_value() {
        let c = curve(0, &[], &[]);
        assert_eq!(sample(&c, 0.3, [1.0, 2.0, 3.0], false), [1.0, 2.0, 3.0]);
    }

    // GFX-GR2-7.5: one knot is a constant; degree 0 is the control of the
    // knot index k = max(1, knots before t).
    #[test]
    fn constant_and_degree_zero() {
        let c = curve(0, &[0.0], &[5.0, 6.0, 7.0]);
        assert_eq!(sample(&c, 9.0, [0.0; 3], false), [5.0, 6.0, 7.0]);
        let held = curve(
            0,
            &[0.0, 1.0, 2.0],
            &[1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0],
        );
        assert_eq!(sample(&held, 0.0, [0.0; 3], false), [2.0; 3]);
        assert_eq!(sample(&held, 1.0, [0.0; 3], false), [2.0; 3]);
        assert_eq!(sample(&held, 1.5, [0.0; 3], false), [3.0; 3]);
    }

    // GFX-GR2-7.5: degree 1 interpolates between the knots' controls.
    #[test]
    fn degree_one() {
        let c = curve(1, &[0.0, 1.0, 3.0], &[1.0, 2.0, 6.0]);
        assert!(close(sample(&c, 0.5, [0.0], false)[0], 1.5));
        assert!(close(sample(&c, 2.0, [0.0], false)[0], 4.0));
        assert!(close(sample(&c, 3.0, [0.0], false)[0], 6.0));
    }

    // GFX-GR2-7.5: degree 2 is the quadratic B-spline on the stored knots,
    // clamped at the ends: the first and last controls at the ends, the
    // inner ones approached, not hit (worked by hand from the formula).
    #[test]
    fn degree_two_on_the_clamped_knots() {
        let c = curve(2, &[0.0, 0.5, 1.0], &[0.0, 2.0, 4.0]);
        assert!(close(sample(&c, 0.0, [9.0], false)[0], 0.0));
        assert!(close(sample(&c, 0.5, [9.0], false)[0], 1.0));
        assert!(close(sample(&c, 0.75, [9.0], false)[0], 2.25));
        assert!(close(sample(&c, 1.0, [9.0], false)[0], 4.0));
    }

    // GFX-GR2-7.5, 7.6: a looping clip's window wraps into the neighbouring
    // period, so the curve runs on across the loop point; clamped, it does
    // not.
    #[test]
    fn looping_windows_wrap() {
        let c = curve(2, &[0.0, 1.0, 2.0, 3.0], &[0.0, 3.0, 1.0, 0.0]);
        let start = sample(&c, 0.0, [0.0], true)[0];
        let end = sample(&c, 3.0, [0.0], true)[0];
        assert!(close(start, end), "{start} {end}");
        let near_start = sample(&c, 0.01, [0.0], true)[0];
        let near_end = sample(&c, 2.99, [0.0], true)[0];
        assert!((near_start - near_end).abs() < 0.1);
        assert!(close(sample(&c, 0.0, [0.0], false)[0], 0.0));
        assert!(!close(start, 0.0));
    }

    // GFX-GR2-7.5 step 4: a blended orientation is scaled by (3 - |q|^2) / 2.
    #[test]
    fn orientations_renormalise_to_first_order() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let c = curve(1, &[0.0, 1.0], &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, h, h]);
        let q = sample(&c, 0.5, [0.0; 4], false);
        let blend = [0.0, 0.0, h / 2.0, 0.5 + h / 2.0];
        let n2: f32 = blend.iter().map(|v| v * v).sum();
        let want = blend.map(|v| v * (3.0 - n2) / 2.0);
        assert!((0..4).all(|i| close(q[i], want[i])), "{q:?}");
    }
}
