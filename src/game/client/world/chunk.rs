
use std::convert::TryInto;

use liquidfun::box2d::dynamics::body::Body;
use mint::Point2;
use sdl2::{pixels::Color, rect::Rect};
use sdl_gpu::{GPUImage, GPURect, GPUSubsystem, GPUTarget, sys::{GPU_FilterEnum, GPU_FormatEnum}};

use crate::game::{client::render::{Fonts, Renderable, Sdl2Context, TransformStack}, common::{Settings, world::{CHUNK_SIZE, Chunk, ChunkHandler, ChunkState, gen::WorldGenerator, material::{MaterialInstance, PhysicsType}}}};

pub struct ClientChunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    pub graphics: Box<ChunkGraphics>,
    pub dirty_rect: Option<Rect>,
    pub b2_body: Option<Body>,
    pub mesh: Option<Vec<Vec<Vec<Vec<f64>>>>>,
    pub mesh_simplified: Option<Vec<Vec<Vec<Vec<f64>>>>>,
    pub tris: Option<Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>>>,
}

impl<'ch> Chunk for ClientChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            graphics: Box::new(ChunkGraphics {
                texture: None,
                pixel_data: [0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)],
                dirty: true,
                was_dirty: true,
            }),
            dirty_rect: None,
            b2_body: None,
            mesh: None,
            mesh_simplified: None,
            tris: None,
        }
    }

    fn get_chunk_x(&self) -> i32 {
        self.chunk_x
    }

    fn get_chunk_y(&self) -> i32 {
        self.chunk_y
    }

    fn get_state(&self) -> ChunkState {
        self.state
    }

    fn set_state(&mut self, state: ChunkState) {
        self.state = state;
    }

    fn get_dirty_rect(&self) -> Option<Rect> {
        self.dirty_rect
    }

    fn set_dirty_rect(&mut self, rect: Option<Rect>) {
        self.dirty_rect = rect;
    }

    fn refresh(&mut self){
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                self.graphics.set(x, y, self.pixels.unwrap()[(x + y * CHUNK_SIZE) as usize].color).unwrap();
            }
        }
    }

    // #[profiling::function]
    fn update_graphics(&mut self) -> Result<(), String> {
        
        self.graphics.was_dirty = self.graphics.dirty;

        self.graphics.update_texture().map_err(|e| format!("ChunkGraphics::update_texture failed: {:?}", e))?;

        Ok(())
    }

    // #[profiling::function] // huge performance impact
    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {

            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                px[i] = mat;
                self.graphics.set(x, y, px[i].color)?;

                self.dirty_rect = Some(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    #[profiling::function]
    fn apply_diff(&mut self, diff: &Vec<(u16, u16, MaterialInstance)>) {
        diff.iter().for_each(|(x, y, mat)| {
            self.set(*x, *y, *mat).unwrap(); // TODO: handle this Err
        });
    }

    fn set_pixels(&mut self, pixels: &[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]) {
        self.pixels = Some(*pixels);

        let c = contour::ContourBuilder::new(CHUNK_SIZE as u32, CHUNK_SIZE as u32, true);
        let vs: Vec<f64> = pixels.iter().map(|p| if p.physics == PhysicsType::Solid {1.0 as f64} else {0.0 as f64}).collect();
        let feat = c.contours(&vs, &[1.0]).map(|vf| match &vf[0].geometry.as_ref().unwrap().value {
            geojson::Value::MultiPolygon(mp) => {
                let mp: Vec<geojson::PolygonType> = mp.to_vec();

                let v: (Vec<Vec<Vec<Vec<f64>>>>, Vec<Vec<Vec<Vec<f64>>>>) = mp.iter().map(|pt| {
                    return pt.iter().map(|ln| {
                        let pts: Vec<Point2<_>> = ln.iter().map(|pt| {
                            let mut x = pt[0];
                            let mut y = pt[1];

                            // this extra manipulation helps seal the seams on chunk edges during the later mesh simplification

                            if (y == 0.0 || y == CHUNK_SIZE as f64) && x == 0.5 {
                                x = 0.0;
                            }

                            if (x == 0.0 || x == CHUNK_SIZE as f64) && y == 0.5 {
                                y = 0.0;
                            }

                            if (y == 0.0 || y == CHUNK_SIZE as f64) && x == CHUNK_SIZE as f64 - 0.5 {
                                x = CHUNK_SIZE as f64;
                            }

                            if (x == 0.0 || x == CHUNK_SIZE as f64) && y == CHUNK_SIZE as f64 - 0.5 {
                                y = CHUNK_SIZE as f64;
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
            _ => unreachable!(),
        });

        if let Ok(r) = feat {
            self.mesh = Some(r.0);
            self.mesh_simplified = Some(r.1);
        }else {
            self.mesh = None;
            self.mesh_simplified = None;
        }


        if let Some(f) = &self.mesh_simplified {
            //Vec<                                         <- parts
            //    Vec<                                     <- tris
            //        ((f64, f64), (f64, f64), (f64, f64)) <- tri
            let r: Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>> = f.iter().map(|part| {

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

                // let mut edges: Vec<(usize, usize)> = part.iter().skip(1).flat_map(|poly| {
                //     let mut v: Vec<(usize, usize)> = Vec::new();
                //     for i in 1..poly.len() {
                //         let (x1, y1) = poly[i-1];
                //         points.push((x1, y1));
                //         let i1 = points.len() - 1;

                //         let (x2, y2) = poly[i];
                //         points.push((x2, y2));
                //         let i2 = points.len() - 1;

                //         v.push((i1, i2));
                //     }
                //     v
                // }).collect();

                // if edges.len() == 0 {
                //     edges = vec![(0, 1)];
                // }

                // let mut edges: Vec<(usize, usize)> = Vec::new();
                // for i in 1..points.len() {
                //     edges.push((i-1, i));
                // }

                // let edges = vec![(0, 1)];
                
                // let r: Result<Vec<(usize, usize, usize)>, cdt::Error> = cdt::Triangulation::build_with_edges(&points, &edges).map(|t| {
                //     t.triangles().collect()
                // });

                // let v: Vec<((f64, f64), (f64, f64), (f64, f64))> = r.map(|r| {
                //     let v: Vec<((f64, f64), (f64, f64), (f64, f64))> = r.iter().map(|tri| 
                //         (points[tri.0], points[tri.1], points[tri.2])
                //     ).collect();
                //     v
                // }).ok().or_else(|| Some(vec![])).unwrap();
                // v
            }).collect();
            self.tris = Some(r);
        }

    }

    fn get_pixels_mut(&mut self) -> &mut Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]> {
        &mut self.pixels
    }

    fn get_pixels(&self) -> &Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]> {
        &self.pixels
    }

    fn set_pixel_colors(&mut self, colors: &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]) {
        self.graphics.replace(*colors);
    }

    fn get_colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.graphics.pixel_data
    }

    fn get_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &self.graphics.pixel_data
    }

    fn mark_dirty(&mut self) {
        self.graphics.dirty = true;
    }

    fn get_tris(&self) -> &Option<Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>>> {
        &self.tris
    }

    fn get_mesh_loops(&self) -> &Option<Vec<Vec<Vec<Vec<f64>>>>> {
        &self.mesh_simplified
    }

    fn get_b2_body(&self) -> &Option<Body> {
        &self.b2_body
    }

    fn get_b2_body_mut(&mut self) -> &mut Option<Body> {
        &mut self.b2_body
    }

    fn set_b2_body(&mut self, body: Option<Body>) {
        self.b2_body = body;
    }
}

pub struct ChunkGraphics {
    pub texture: Option<GPUImage>,
    pub pixel_data: [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4],
    pub dirty: bool,
    pub was_dirty: bool,
}

impl<'cg> ChunkGraphics {
    // #[profiling::function] // huge performance impact
    pub fn set(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            // self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            let i = (x + y * CHUNK_SIZE) as usize;
            self.pixel_data[i * 4 + 0] = color.r;
            self.pixel_data[i * 4 + 1] = color.g;
            self.pixel_data[i * 4 + 2] = color.b;
            self.pixel_data[i * 4 + 3] = color.a;
            self.dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    // #[profiling::function]
    pub fn update_texture(&mut self) -> Result<(), ()> {
        if self.dirty {
            if self.texture.is_none() {
                self.texture = Some(GPUSubsystem::create_image(CHUNK_SIZE, CHUNK_SIZE, GPU_FormatEnum::GPU_FORMAT_RGBA));
                self.texture.as_mut().unwrap().set_image_filter(GPU_FilterEnum::GPU_FILTER_NEAREST);
            }
            self.texture.as_mut().unwrap().update_image_bytes(None as Option<GPURect>, &self.pixel_data, (CHUNK_SIZE * 4).into());
            self.dirty = false;
        }

        Ok(())
    }

    #[profiling::function]
    pub fn replace(&mut self, colors: [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]){
        // let sf = Surface::from_data(&mut colors, CHUNK_SIZE as u32, CHUNK_SIZE as u32, self.surface.pitch(), self.surface.pixel_format_enum()).unwrap();
        // sf.blit(None, &mut self.surface, None).unwrap();
        self.pixel_data = colors;
        self.dirty = true;
    }
}

impl Renderable for ClientChunk {
    fn render(&self, canvas : &mut GPUTarget, transform: &mut TransformStack, sdl: &Sdl2Context, fonts: &Fonts, settings: &Settings) {
        self.graphics.render(canvas, transform, sdl, fonts, settings);

        if settings.debug && settings.draw_chunk_collision == 1 {
            if let Some(f) = &self.mesh {
                
                let colors = vec![
                    Color::RGB(32, 255, 32),
                    Color::RGB(255, 32, 32),
                    Color::RGB(32, 64, 255),
                    Color::RGB(255, 255, 32),
                    Color::RGB(32, 255, 255),
                    Color::RGB(255, 32, 255),
                ];

                f.iter().enumerate().for_each(|(j, f)| {
                    f.iter().enumerate().for_each(|(k, pts)| {
                        for i in 1..pts.len() {
                            let (x1, y1) = transform.transform((pts[i-1][0], pts[i-1][1]));
                            let (x2, y2) = transform.transform((pts[i][0], pts[i][1]));
                            canvas.line(x1 as f32, y1 as f32, x2 as f32, y2 as f32, colors[j % colors.len()]);
                        }

                        // draw individual points
                        // for i in 0..pts.len() {
                        //     let (x1, y1) = transform.transform((pts[i][0], pts[i][1]));
                        //     canvas.rectangle(x1 as f32 - 1.0, y1 as f32 - 1.0, x1 as f32 + 1.0, y1 as f32 + 1.0, colors[(j + k) % colors.len()]);
                        // }
                    });
                });
            }
        }else if settings.debug && settings.draw_chunk_collision == 2 {
            if let Some(f) = &self.mesh_simplified {
                
                let colors = vec![
                    Color::RGB(32, 255, 32),
                    Color::RGB(255, 32, 32),
                    Color::RGB(32, 64, 255),
                    Color::RGB(255, 255, 32),
                    Color::RGB(32, 255, 255),
                    Color::RGB(255, 32, 255),
                ];

                f.iter().enumerate().for_each(|(j, f)| {
                    f.iter().enumerate().for_each(|(_k, pts)| {
                        for i in 1..pts.len() {
                            let (x1, y1) = transform.transform((pts[i-1][0], pts[i-1][1]));
                            let (x2, y2) = transform.transform((pts[i][0], pts[i][1]));
                            canvas.line(x1 as f32, y1 as f32, x2 as f32, y2 as f32, colors[j % colors.len()]);
                        }
                    });
                });
            }
        }else if settings.debug && settings.draw_chunk_collision == 3 {
            if let Some(t) = &self.tris {
                t.iter().for_each(|part| {
                    part.iter().for_each(|tri| {
                        let (x1, y1) = transform.transform(tri.0);
                        let (x2, y2) = transform.transform(tri.1);
                        let (x3, y3) = transform.transform(tri.2);

                        let color = Color::RGBA(32, 255, 255, 255);

                        canvas.line(x1 as f32, y1 as f32, x2 as f32, y2 as f32, color);
                        canvas.line(x2 as f32, y2 as f32, x3 as f32, y3 as f32, color);
                        canvas.line(x3 as f32, y3 as f32, x1 as f32, y1 as f32, color);
                    });
                });
            }
        }
    }
}

impl Renderable for ChunkGraphics {
    fn render(&self, target : &mut GPUTarget, transform: &mut TransformStack, _sdl: &Sdl2Context, _fonts: &Fonts, _settings: &Settings) {
        let chunk_rect = transform.transform_rect(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

        if let Some(tex) = &self.texture {
            tex.blit_rect(None, target, Some(chunk_rect));
        }else{
            target.rectangle_filled2(chunk_rect, Color::RGB(127, 0, 0));
        }
    }
}

impl<T: WorldGenerator + Copy + Send + Sync + 'static> ChunkHandler<T, ClientChunk> {
    pub fn sync_chunk(&mut self, chunk_x: i32, chunk_y: i32, pixels: Vec<MaterialInstance>, colors: Vec<u8>) -> Result<(), String>{
        if pixels.len() != (CHUNK_SIZE * CHUNK_SIZE) as usize {
            return Err(format!("pixels Vec is the wrong size: {} (expected {})", pixels.len(), CHUNK_SIZE * CHUNK_SIZE));
        }

        if colors.len() != CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4 {
            return Err(format!("colors Vec is the wrong size: {} (expected {})", colors.len(), CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4));
        }

        if let Some(chunk) = self.loaded_chunks.get_mut(&self.chunk_index(chunk_x, chunk_y)) {
            chunk.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
        }else{
            let mut chunk: ClientChunk = Chunk::new_empty(chunk_x, chunk_y);
            chunk.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
            self.loaded_chunks.insert(self.chunk_index(chunk_x, chunk_y), Box::new(chunk));
        }

        Ok(())
    }
}