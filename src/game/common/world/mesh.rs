use mint::Point2;

use super::material::{MaterialInstance, PhysicsType};

pub fn pixels_to_valuemap(pixels: &[MaterialInstance]) -> Vec<f64> {
    pixels.iter().map(|p| {
        if p.physics == PhysicsType::Solid {
            1.0 as f64
        } else {
            0.0 as f64
        }
    }).collect()
}

pub fn generate_mesh_only_simplified(values: Vec<f64>, width: u32, height: u32) -> Result<Vec<Vec<Vec<Vec<f64>>>>, String> {
    generate_mesh_with_simplified(values, width, height).map(|t| t.1)
}

#[allow(dead_code)]
pub fn generate_mesh_only_unsimplified(values: Vec<f64>, width: u32, height: u32) -> Result<Vec<Vec<Vec<Vec<f64>>>>, String> {
    generate_mesh_with_simplified(values, width, height).map(|t| t.0)
}

pub fn generate_mesh_with_simplified(values: Vec<f64>, width: u32, height: u32) -> Result<(Vec<Vec<Vec<Vec<f64>>>>, Vec<Vec<Vec<Vec<f64>>>>), String> {
    if values.len() as u32 != width * height {
        return Err(format!("generate_mesh failed: Dimension mismatch (w*h = {}*{} = {}, but values.len() = {})", width, height, width * height, values.len() as u32));
    }

    let c = contour::ContourBuilder::new(width, height, true);

    let contours = c.contours(&values, &[1.0]);

    let feat = contours.map(|vf| {
        // this unwrap should never fail, since the Features returned by contours are always Some geometry with MultiPolygon value
        match &vf[0].geometry.as_ref().unwrap().value {
            geojson::Value::MultiPolygon(mp) => {
                let v: (Vec<Vec<Vec<Vec<f64>>>>, Vec<Vec<Vec<Vec<f64>>>>) = mp.iter().map(|pt| {
                    return pt.iter().map(|ln| {
                        let pts: Vec<Point2<_>> = ln.iter().map(|pt| {
                            let mut x = pt[0];
                            let mut y = pt[1];

                            // this extra manipulation helps seal the seams on chunk edges during the later mesh simplification

                            if (y == 0.0 || y == height as f64) && x == 0.5 {
                                x = 0.0;
                            }

                            if (x == 0.0 || x == width as f64) && y == 0.5 {
                                y = 0.0;
                            }

                            if (y == 0.0 || y == height as f64) && x == width as f64 - 0.5 {
                                x = width as f64;
                            }

                            if (x == 0.0 || x == width as f64) && y == height as f64 - 0.5 {
                                y = height as f64;
                            }

                            x = x.round() - 0.5;
                            y = y.round() - 0.5;

                            Point2{
                                x,
                                y,
                            }
                        }).collect();

                        let keep = ramer_douglas_peucker::rdp(&pts, 1.0);

                        let p1: Vec<Vec<f64>> = pts.iter().map(|p| vec![p.x, p.y]).collect();
                        let p2: Vec<Vec<f64>> = pts.iter().enumerate().filter(|(i, &_p)| {
                            keep.contains(i)
                        }).map(|(_, p)| vec![p.x, p.y]).collect();
                        return (p1, p2);
                    }).filter(|(norm, simple)| norm.len() > 2 && simple.len() > 2).unzip();
                }).filter(|p: &(Vec<Vec<Vec<f64>>>, Vec<Vec<Vec<f64>>>)| p.0.len() > 0 && p.1.len() > 0).unzip();
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
pub fn triangulate(mesh: &Vec<Vec<Vec<Vec<f64>>>>) -> Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>> {
    mesh.iter().map(|part| {

        let (vertices, holes, dimensions) = earcutr::flatten(part);
        let triangles = earcutr::earcut(&vertices, &holes, dimensions);

        let mut res: Vec<((f64, f64), (f64, f64), (f64, f64))> = Vec::new();

        for i in (0..triangles.len()).step_by(3) {
            let a = (vertices[triangles[i  ] * 2], vertices[triangles[i  ] * 2 + 1]);
            let b = (vertices[triangles[i+1] * 2], vertices[triangles[i+1] * 2 + 1]);
            let c = (vertices[triangles[i+2] * 2], vertices[triangles[i+2] * 2 + 1]);
            res.push((a, b, c));
        }

        res
    }).collect()
}

