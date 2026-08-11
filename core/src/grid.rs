use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridIndex {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub trait GridSystem: Send + Sync {
    /// Convert screen/world coordinates to a grid index
    fn world_to_grid(&self, point: Point) -> Option<GridIndex>;

    /// Get the vertices defining the shape of a specific cell
    fn cell_geometry(&self, index: GridIndex) -> Vec<Point>;

    /// Get neighboring cells (for flood fill, etc.)
    fn neighbors(&self, index: GridIndex) -> Vec<GridIndex>;

    /// Get the center point of a cell
    fn cell_center(&self, index: GridIndex) -> Point;
}

#[derive(Debug, Clone)]
pub struct SquareGrid {
    pub cell_size: f32,
}

impl SquareGrid {
    pub fn new(cell_size: f32) -> Self {
        Self { cell_size }
    }
}

impl GridSystem for SquareGrid {
    fn world_to_grid(&self, point: Point) -> Option<GridIndex> {
        let x = (point.x / self.cell_size).floor() as i32;
        let y = (point.y / self.cell_size).floor() as i32;
        Some(GridIndex { x, y })
    }

    fn cell_geometry(&self, index: GridIndex) -> Vec<Point> {
        let x = index.x as f32 * self.cell_size;
        let y = index.y as f32 * self.cell_size;
        let s = self.cell_size;

        vec![
            Point::new(x, y),
            Point::new(x + s, y),
            Point::new(x + s, y + s),
            Point::new(x, y + s),
        ]
    }

    fn neighbors(&self, index: GridIndex) -> Vec<GridIndex> {
        vec![
            GridIndex {
                x: index.x + 1,
                y: index.y,
            },
            GridIndex {
                x: index.x - 1,
                y: index.y,
            },
            GridIndex {
                x: index.x,
                y: index.y + 1,
            },
            GridIndex {
                x: index.x,
                y: index.y - 1,
            },
        ]
    }

    fn cell_center(&self, index: GridIndex) -> Point {
        let x = index.x as f32 * self.cell_size + self.cell_size / 2.0;
        let y = index.y as f32 * self.cell_size + self.cell_size / 2.0;
        Point::new(x, y)
    }
}

#[derive(Debug, Clone)]
pub struct TriangleGrid {
    pub cell_size: f32,
}

impl TriangleGrid {
    pub fn new(cell_size: f32) -> Self {
        Self { cell_size }
    }

    fn width(&self) -> f32 {
        self.cell_size / 2.0
    }

    fn height(&self) -> f32 {
        self.cell_size * 3.0f32.sqrt() / 2.0
    }
}

impl GridSystem for TriangleGrid {
    fn world_to_grid(&self, point: Point) -> Option<GridIndex> {
        let w = self.width();
        let h = self.height();

        let gy = (point.y / h).floor() as i32;
        let gx = (point.x / w).floor() as i32;

        let u = (point.x - gx as f32 * w) / w;
        let v = (point.y - gy as f32 * h) / h;

        let even_sum = (gx + gy) % 2 == 0;

        let index_x = if even_sum {
            // Diagonal v = u
            if v < u { gx } else { gx - 1 }
        } else {
            // Diagonal v = 1 - u
            if u + v > 1.0 { gx } else { gx - 1 }
        };

        Some(GridIndex { x: index_x, y: gy })
    }

    fn cell_geometry(&self, index: GridIndex) -> Vec<Point> {
        let w = self.width();
        let h = self.height();

        // Parity based on logic: Even sum -> Down, Odd sum -> Up
        let is_up = (index.x + index.y) % 2 != 0;

        let x = index.x as f32;
        let y = index.y as f32;

        if is_up {
            // Up Triangle
            // Tip: (x*w + w, y*h)
            // Base Left: (x*w, (y+1)*h)
            // Base Right: (x*w + 2w, (y+1)*h)
            vec![
                Point::new(x * w + w, y * h),
                Point::new(x * w + 2.0 * w, (y + 1.0) * h),
                Point::new(x * w, (y + 1.0) * h),
            ]
        } else {
            // Down Triangle
            // Base Left: (x*w, y*h)
            // Base Right: (x*w + 2w, y*h)
            // Tip: (x*w + w, (y+1)*h)
            vec![
                Point::new(x * w, y * h),
                Point::new(x * w + 2.0 * w, y * h),
                Point::new(x * w + w, (y + 1.0) * h),
            ]
        }
    }

    fn neighbors(&self, index: GridIndex) -> Vec<GridIndex> {
        let is_up = (index.x + index.y) % 2 != 0;

        let mut n = vec![
            GridIndex {
                x: index.x - 1,
                y: index.y,
            },
            GridIndex {
                x: index.x + 1,
                y: index.y,
            },
        ];

        if is_up {
            n.push(GridIndex {
                x: index.x,
                y: index.y + 1,
            });
        } else {
            n.push(GridIndex {
                x: index.x,
                y: index.y - 1,
            });
        }
        n
    }

    fn cell_center(&self, index: GridIndex) -> Point {
        let w = self.width();
        let h = self.height();
        let is_up = (index.x + index.y) % 2 != 0;

        let x = index.x as f32 * w + w; // Center x is same for both
        let y = if is_up {
            index.y as f32 * h + 2.0 * h / 3.0
        } else {
            index.y as f32 * h + h / 3.0
        };

        Point::new(x, y)
    }
}

#[derive(Debug, Clone)]
pub struct HexagonGrid {
    pub cell_size: f32, // Outer radius
}

impl HexagonGrid {
    pub fn new(cell_size: f32) -> Self {
        Self { cell_size }
    }
}

impl GridSystem for HexagonGrid {
    fn world_to_grid(&self, point: Point) -> Option<GridIndex> {
        let size = self.cell_size;

        // Pointy-topped conversion
        let q = (3.0f32.sqrt() / 3.0 * point.x - 1.0 / 3.0 * point.y) / size;
        let r = (2.0 / 3.0 * point.y) / size;
        let s = -q - r;

        // Cube rounding
        let mut rx = q.round();
        let mut ry = r.round();
        let rz = s.round();

        let x_diff = (rx - q).abs();
        let y_diff = (ry - r).abs();
        let z_diff = (rz - s).abs();

        if x_diff > y_diff && x_diff > z_diff {
            rx = -ry - rz;
        } else if y_diff > z_diff {
            ry = -rx - rz;
        }

        Some(GridIndex {
            x: rx as i32,
            y: ry as i32,
        })
    }

    fn cell_geometry(&self, index: GridIndex) -> Vec<Point> {
        let size = self.cell_size;
        let q = index.x as f32;
        let r = index.y as f32;

        let cx = size * (3.0f32.sqrt() * q + 3.0f32.sqrt() / 2.0 * r);
        let cy = size * (3.0 / 2.0 * r);

        (0..6)
            .map(|i| {
                let angle = (60.0 * i as f32 - 30.0).to_radians();
                Point::new(cx + size * angle.cos(), cy + size * angle.sin())
            })
            .collect()
    }

    fn neighbors(&self, index: GridIndex) -> Vec<GridIndex> {
        // Axial neighbors
        let dirs = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

        dirs.iter()
            .map(|(dx, dy)| GridIndex {
                x: index.x + dx,
                y: index.y + dy,
            })
            .collect()
    }

    fn cell_center(&self, index: GridIndex) -> Point {
        let size = self.cell_size;
        let q = index.x as f32;
        let r = index.y as f32;

        Point::new(
            size * (3.0f32.sqrt() * q + 3.0f32.sqrt() / 2.0 * r),
            size * (3.0 / 2.0 * r),
        )
    }
}
