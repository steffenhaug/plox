use crate::line::Segment;
use std::f32::consts::PI;

#[allow(unused_assignments)]
pub fn tesselate<'a, S>(mut segments: S, width: f32) -> (Vec<glm::Vec2>, Vec<glm::Vec2>, Vec<u32>)
where
    S: Iterator<Item = &'a Segment>,
{
    // Consider a pair of segments at a time
    let mut s1 = None;
    let mut s2 = None;

    // Build a vertex- and index-buffer.
    let mut verts = Vec::with_capacity(256);
    let mut uvs = Vec::with_capacity(256);
    let mut length = 0.0;

    // π/2 rotation matrix.
    let rot = glm::mat2(0.0, -1.0, 1.0, 0.0);

    loop {
        s1 = s2;
        s2 = segments.next();

        match (s1, s2) {
            (None, Some(s)) => {
                let Segment { p1, .. } = s;
                let v = s.vec();
                let l = v.norm();

                // vw-basis for the segments local space (see report).
                let v = v / l;
                let w = rot * v;

                // Calculate the new vertices.
                let v1 = (-width * v) - (width * w) + p1;
                let v2 = (-width * v) + (width * w) + p1;
                verts.extend([v1, v2]);

                // Calculate uv-coordinates
                let uv1 = glm::vec2(0.0, 0.0);
                let uv2 = glm::vec2(0.0, 1.0);
                uvs.extend([uv1, uv2]);
            }
            (Some(s), None) => {
                let Segment { p2, .. } = s;
                let v = s.vec();
                let l = v.norm();
                length += l;

                // vw-basis for the segments local space (see report).
                let v = v / l;
                let w = rot * v;

                // Calculate the new vertices.
                let v1 = (width * v) - (width * w) + p2;
                let v2 = (width * v) + (width * w) + p2;
                verts.extend([v1, v2]);

                // Calculate uv-coordinates
                let uv1 = glm::vec2(length, 0.0);
                let uv2 = glm::vec2(length, 1.0);
                uvs.extend([uv1, uv2]);
            }
            (Some(s1), Some(s2)) => {
                let Segment { p1: p2, .. } = s2;
                let v = s1.vec();
                let l = v.norm();
                length += l;

                // vw-basis for the first segments local space (see report).
                let v = v / l;
                let w = rot * v;

                // Find the angle the 2nd line segment is turning at.
                let u = s2.vec();
                let th = v.angle(&u);
                let phi = (PI - th) / 2.0;
                // Calculate the consequtent v-delta.
                // Note, this has problems: tan(x) explodes
                let dv = width * f32::tan(PI / 2.0 - phi);
                // Note that which "side" to add and take away from depends
                // on which way the line is curving (obviously). The sign of
                // u's w-component gives this away.
                let s = f32::signum(w.dot(&u));

                let v1 = s * (dv * v) - (width * w) + p2;
                let v2 = -s * (dv * v) + (width * w) + p2;
                verts.extend([v1, v2]);

                // Calculate uv-coordinates
                let uv1 = glm::vec2(length, 0.0);
                let uv2 = glm::vec2(length, 1.0);
                uvs.extend([uv1, uv2]);
            }
            (None, None) => break,
        }
    }

    // Create the index buffer.
    // A line of N segments has 2(N+1) vertices.
    let idx = (0..verts.len() as u32 / 2 - 1)
        .flat_map(|n| {
            // The N-th line segment has indeces
            //   2N 2N+2 2N+3
            //   2N 2N+3 2N+1
            [2 * n, 2 * n + 2, 2 * n + 3, 2 * n, 2 * n + 3, 2 * n + 1]
        })
        .collect();

    // Normalize the uv-coordinates.
    for uv in uvs.iter_mut() {
        uv.x /= length;
    }

    (verts, uvs, idx)
}
