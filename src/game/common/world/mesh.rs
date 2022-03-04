use mint::Point2;

use super::material::{MaterialInstance, PhysicsType};

pub type Tri = ((f64, f64), (f64, f64), (f64, f64));

pub type VPoint = Vec<f64>;
pub type Poly = Vec<VPoint>;
pub type Loop = Vec<Poly>;
pub type Mesh = Vec<Loop>;

pub fn pixels_to_valuemap(pixels: &[MaterialInstance]) -> Vec<f64> {
    pixels
        .iter()
        .map(|p| {
            if p.physics == PhysicsType::Solid {
                1.0
            } else {
                0.0
            }
        })
        .collect()
}

pub fn generate_mesh_only_simplified(
    values: &[f64],
    width: u32,
    height: u32,
) -> Result<Mesh, String> {
    generate_mesh_with_simplified(values, width, height).map(|t| t.1)
}

#[allow(dead_code)]
pub fn generate_mesh_only_unsimplified(
    values: &[f64],
    width: u32,
    height: u32,
) -> Result<Mesh, String> {
    generate_mesh_with_simplified(values, width, height).map(|t| t.0)
}

pub fn generate_mesh_with_simplified(
    values: &[f64],
    width: u32,
    height: u32,
) -> Result<(Mesh, Mesh), String> {
    if values.len() as u32 != width * height {
        return Err(format!(
            "generate_mesh failed: Dimension mismatch (w*h = {}*{} = {}, but values.len() = {})",
            width,
            height,
            width * height,
            values.len() as u32
        ));
    }

    let c = contour::ContourBuilder::new(width, height, true);

    let contours = c.contours(values, &[1.0]);

    let feat = contours.map(|vf| {
        // this unwrap should never fail, since the Features returned by contours are always Some geometry with MultiPolygon value
        match &vf[0].geometry.as_ref().unwrap().value {
            geojson::Value::MultiPolygon(mp) => {
                let v: (Mesh, Mesh) = mp
                    .iter()
                    .map(|pt| {
                        return pt
                            .iter()
                            .map(|ln| {
                                let pts: Vec<Point2<_>> = ln
                                    .iter()
                                    .map(|pt| {
                                        let mut x = pt[0];
                                        let mut y = pt[1];

                                        // this extra manipulation helps seal the seams on chunk edges during the later mesh simplification

                                        if (y == 0.0
                                            || (y - f64::from(height)).abs() < f64::EPSILON)
                                            && (x - 0.5).abs() < f64::EPSILON
                                        {
                                            x = 0.0;
                                        }

                                        if (x == 0.0 || (x - f64::from(width)).abs() < f64::EPSILON)
                                            && (y - 0.5).abs() < f64::EPSILON
                                        {
                                            y = 0.0;
                                        }

                                        if (y == 0.0
                                            || (y - f64::from(height)).abs() < f64::EPSILON)
                                            && (x - (f64::from(width) - 0.5)).abs() < f64::EPSILON
                                        {
                                            x = f64::from(width);
                                        }

                                        if (x == 0.0 || (x - f64::from(width)).abs() < f64::EPSILON)
                                            && (y - (f64::from(height) - 0.5)).abs() < f64::EPSILON
                                        {
                                            y = f64::from(height);
                                        }

                                        x = x.round() - 0.5;
                                        y = y.round() - 0.5;

                                        Point2 { x, y }
                                    })
                                    .collect();

                                let keep = ramer_douglas_peucker::rdp(&pts, 1.0);

                                let p1: Poly = pts.iter().map(|p| vec![p.x, p.y]).collect();
                                let p2: Poly = pts
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, &_p)| keep.contains(i))
                                    .map(|(_, p)| vec![p.x, p.y])
                                    .collect();
                                (p1, p2)
                            })
                            .filter(|(norm, simple)| norm.len() > 2 && simple.len() > 2)
                            .unzip();
                    })
                    .filter(|p: &(Loop, Loop)| !p.0.is_empty() && !p.1.is_empty())
                    .unzip();
                v
            },
            _ => unreachable!(), // it is always a MultiPolygon
        }
    });

    feat.map_err(|e| e.to_string())
}

/// return type:<br>
/// Vec<                                         -- parts<br>
///     Vec<                                     -- tris<br>
///         ((f64, f64), (f64, f64), (f64, f64)) -- tri
#[allow(clippy::ptr_arg)]
pub fn triangulate(mesh: &Mesh) -> Vec<Vec<Tri>> {
    mesh.iter()
        .map(|part| {
            let (vertices, holes, dimensions) = earcutr::flatten(part);
            let triangles = earcutr::earcut(&vertices, &holes, dimensions);

            let mut res: Vec<Tri> = Vec::new();

            for i in (0..triangles.len()).step_by(3) {
                let a = (vertices[triangles[i] * 2], vertices[triangles[i] * 2 + 1]);
                let b = (
                    vertices[triangles[i + 1] * 2],
                    vertices[triangles[i + 1] * 2 + 1],
                );
                let c = (
                    vertices[triangles[i + 2] * 2],
                    vertices[triangles[i + 2] * 2 + 1],
                );
                res.push((a, b, c));
            }

            res
        })
        .collect()
}
