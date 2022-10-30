use fontdue::Font;
use rayon::prelude::*;
use std::{io::{Error, ErrorKind, Write}, fs::File};
use image::{GenericImage, GenericImageView, ImageBuffer, RgbImage, Pixel as Pxl};

/// Example usage for this trait:
/// ```
/// let blocks_by_darkness = vec![(255, "▓"), (170, "▒"), (85, "░"), (0, " ")];
/// let block_map: std::collections::HashMap<u8, &str> = blocks_by_darkness.into_iter().collect();
/// let number = 240;
/// block_map.closest_value(&number).unwrap() //should return "▓"
/// ```

trait Distance<K, V>
where
    V: Clone,
    K: Clone + PartialOrd + std::ops::Sub + Into<f64> + std::ops::Sub<Output = K>,
    f64: From<K>,
    Self: IntoIterator<Item = (K, V)> + Sized + Clone,
{
    fn sort_self(&self, key: K) -> Vec<(K, V)> {
        let mut self_as_slice: Vec<(K, V)> = self.clone().into_iter().collect::<Vec<_>>();
        self_as_slice.sort_by(|(k1, _), (k2, _)| {
            ((f64::from(k1.clone()) - f64::from(key.clone())).abs())
                .partial_cmp(&(f64::from(k2.clone()) - f64::from(key.clone())).abs())
                .unwrap()
        });
        self_as_slice
    }
    fn closest_value(&self, key: K) -> Option<V> {
        let mut closest_value = self.sort_self(key).get(0).map(|(_, v)| v.clone());
        closest_value
    }
    fn closest_key(&self, key: K) -> Option<K> {
        let mut closest_key = self.sort_self(key).get(0).map(|(k, _)| k.clone());
        closest_key
    }
}

impl<K, V> Distance<K, V> for std::collections::HashMap<K, V>
where
    V: Clone,
    K: Clone + PartialOrd + std::ops::Sub + Into<f64> + std::ops::Sub<Output = K>,
    f64: From<K>,
{
}

trait Rows<T> {
    fn rows(self, length: usize) -> Vec<Vec<T>>;
}

impl<T> Rows<T> for Vec<T> {
    fn rows(self, length: usize) -> Vec<Vec<T>> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        for item in self {
            row.push(item);
            if row.len() == length {
                rows.push(row);
                row = Vec::new();
            }
        }
        rows
    }
}

// Takes a width, a height, and (x, y) coordinates and returns whether the coordinates are in the bounds of the matrix
fn in_bounds(width: usize, height: usize, xy: (usize, usize)) -> bool {
    let (x, y) = xy;
    x < width && y < height
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Quadrants<T: Clone + Copy> {
    top_left: T,
    top_right: T,
    bottom_left: T,
    bottom_right: T,
}

impl<T> Quadrants<T> where T: Clone + Copy + std::ops::Sub + Into<f64> + std::ops::Sub<Output = T>{
    fn distance(&self, other: &Self) -> f64 {
        let top_left_distance: f64 = (self.top_left.into() - other.top_left.into()).abs().powf(2.0);
        let top_right_distance = (self.top_right.into() - other.top_right.into()).abs().powf(2.0);
        let bottom_left_distance = (self.bottom_left.into() - other.bottom_left.into()).abs().powf(2.0);
        let bottom_right_distance = (self.bottom_right.into() - other.bottom_right.into()).abs().powf(2.0);
        ((top_left_distance + top_right_distance + bottom_left_distance + bottom_right_distance) / 4.0).sqrt()
    }
}

// Takes a matrix of type T, such as a nested array of pixel values. Safely produces a Vec<Vec<Quadrants<T>>> with proper checking this time and zero unwrapping. You could use this to produce 4x4 pixel areas, for instance.
fn into_quadrants<'a, T: Copy>(
    matrix: Vec<Vec<T>>,
) -> Result<Vec<Vec<Quadrants<T>>>, Box<dyn std::error::Error>> {
    let bounds_err = || {
        Error::new(
            ErrorKind::InvalidInput,
            format!(
                "The selected row is too short to contain the pixel"
            ),
        )
    };
    let width = matrix
        .get(0)
        .ok_or(Error::new(ErrorKind::InvalidInput, "Matrix is empty"))?
        .len();
    let height = matrix.len();
    let mut quadrant_rows = Vec::new();
    for row_idx in (0..height).step_by(2) {
        if row_idx + 1 >= height {
            break
        }
        let row = matrix.get(row_idx).ok_or(bounds_err())?;
        let mut quadrant_cols = Vec::new();
        for col_idx in (0..width).step_by(2) {
            if col_idx + 1 >= width {
                break
            }
            let result: Result<Quadrants<T>, Box<dyn std::error::Error>> = {
                let top_left = row.get(col_idx).ok_or(bounds_err())?;
                let top_right = row.get(col_idx + 1).ok_or(bounds_err())?;
                let bottom_left = matrix.get(row_idx + 1).ok_or(bounds_err())?.get(col_idx).ok_or(bounds_err())?;
                let bottom_right = matrix.get(row_idx + 1).ok_or(bounds_err())?.get(col_idx + 1).ok_or(bounds_err())?;
                Ok(Quadrants {
                    top_left: *top_left,
                    top_right: *top_right,
                    bottom_left: *bottom_left,
                    bottom_right: *bottom_right,
                })
            };
            if result.is_ok() {
                quadrant_cols.push(result.unwrap());
            }
        }
        quadrant_rows.push(quadrant_cols);
    }
    Ok(quadrant_rows)
}

// Reads in a png file as Luma values, converts it to a Vec<Vec<Quadrants<u8>>> and returns it.
fn read_png_into_quadrants(path: &str) -> Result<Vec<Vec<Quadrants<u8>>>, Box<dyn std::error::Error>> {
    use std::io::Cursor;
    use image::io::Reader as ImageReader;
    let img = ImageReader::open(path)?.decode()?;
    let width = img.width();
    let height = img.height();
    let mut flattened = Vec::new();
    for group in img.pixels() {
        flattened.push(group.2.0[2]) ; // the alpha values
    }
    into_quadrants(flattened.rows(width as usize))
}

// Basic CSS-adjacent enums.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VertAlignment {
    Top,
    Bottom,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorzAlignment {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    vert: VertAlignment,
    horz: HorzAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentage(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pixel(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimensionValue {
    Percentage(Percentage),
    Pixel(Pixel),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dimensions {
    pub width: DimensionValue,
    pub height: DimensionValue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Div {
    pub dimensions: Dimensions,
    pub alignment: Alignment,
    pub luminosity: u8,
}

// A macro for creating divs.
macro_rules! div {
    (w: $width:expr, h: $height:expr, vrt: $vert:ident, hrz: $horz:ident, unit: $unit:tt, lum: $lum:expr) => {
        Div {
            dimensions: Dimensions {
                width: div!($width, $unit),
                height: div!($height, $unit),
            },
            alignment: Alignment {
                vert: div!(VertAlignment, $vert),
                horz: div!(HorzAlignment, $horz),
            },
            luminosity: $lum as u8,
        }
    };
    ($alignmenttype:ident, top) => {
        $alignmenttype::Top
    };
    ($alignmenttype:ident, bottom) => {
        $alignmenttype::Bottom
    };
    ($alignmenttype:ident, center) => {
        $alignmenttype::Center
    };
    ($alignmenttype:ident, left) => {
        $alignmenttype::Left
    };
    ($alignmenttype:ident, right) => {
        $alignmenttype::Right
    };
    ($dim:expr, px) => {
        DimensionValue::Pixel(Pixel($dim as f64))
    };
    ($dim:expr, %) => {
        DimensionValue::Percentage(Percentage($dim as f64))
    };
}

#[test]
fn test_div_macro() {
    let div = div!(
        w: 100,
        h: 200,
        vrt: top,
        hrz: left,
        unit: px,
        lum: 255
    );
}

// Takes the width and the height of the console, and a Div. First, it allocates a matrix of Quadrants<u8> with values all set to 0. Then it draws the Div on the matrix. Finally, it returns the matrix.
fn draw_div(mut matrix: Vec<Vec<Quadrants<u8>>>, width: usize, height: usize, div: Div) -> Vec<Vec<Quadrants<u8>>> {
    let (dimensions, alignment) = (div.dimensions, div.alignment);
    let (w, h) = (dimensions.width, dimensions.height);
    let (horz, vert) = (alignment.horz, alignment.vert);
    let (abs_width) = match w {
        DimensionValue::Percentage(Percentage(p)) => {
            let w: f64 = (p / 100.0) * width as f64;
            w as usize
        }
        DimensionValue::Pixel(Pixel(p)) => p as usize,
    };
    let (abs_height) = match h {
        DimensionValue::Percentage(Percentage(p)) => {
            let h: f64 = (p / 100.0) * height as f64;
            h as i64
        }
        DimensionValue::Pixel(Pixel(p)) => p as i64,
    };
    let (abs_x, abs_y) = match horz {
        HorzAlignment::Left => (0 as i64, 0 as i64),
        HorzAlignment::Right => (width as i64 - abs_width as i64, 0 as i64),
        HorzAlignment::Center => ((width as i64 - abs_width as i64) / 2 as i64, 0 as i64),
    };
    let (abs_x, abs_y) = match vert {
        VertAlignment::Top => (abs_x, 0),
        VertAlignment::Bottom => (abs_x, height as i64 - abs_height as i64),
        VertAlignment::Center => (abs_x, (height as i64 - abs_height as i64) / 2 as i64),
    };
    let is_within_box = move |x: usize, y: usize| {
        let xpos = x as i64;
        let ypos = y as i64;
        let x_within_box = xpos >= abs_x && xpos < abs_x + abs_width as i64;
        let y_within_box = ypos >= abs_y && ypos < abs_y + abs_height as i64;
        x_within_box && y_within_box
    };
    // Iterate over the (x, y) coordinates of the matrix, replace the empty Quadrant with a Quadrant { top_left: 255, top_right: 255, bottom_left: 255, bottom_right: 255 }.
    for y in 0..height {
        for x in 0..width {
            if is_within_box(x, y) {
                matrix[y][x] = Quadrants {
                    top_left: div.luminosity,
                    top_right: div.luminosity,
                    bottom_left: div.luminosity,
                    bottom_right: div.luminosity,
                };
            }
        }
    }
    matrix
}
fn draw_div_from_scratch(width: usize, height: usize, div: Div) -> Vec<Vec<Quadrants<u8>>> {
    let mut matrix = vec![vec![Quadrants {
        top_left: 0,
        top_right: 0,
        bottom_left: 0,
        bottom_right: 0,
    }; width]; height];
    draw_div(matrix, width, height, div)
}
fn convert_div_to_matrix(div: Div) -> Vec<Vec<Quadrants<u8>>> {
    let (width, height) = (div.dimensions.width, div.dimensions.height);
    let (w, h) = match (width, height) {
        (DimensionValue::Pixel(Pixel(p1)), DimensionValue::Pixel(Pixel(p2))) => (p1, p2),
        _ => panic!("Div dimensions must be pixels."),
    };
    let matrix = vec![vec![Quadrants {
        top_left: 0,
        top_right: 0,
        bottom_left: 0,
        bottom_right: 0,
    }; w as usize]; h as usize];
    draw_div(matrix, w as usize, h as usize, div)
}



#[test]
fn test_into_quadrants() {
    let matrix = (0..7).map(|i| (0..7).collect::<Vec<_>>()).collect::<Vec<_>>();
    for row in matrix.iter() {
        println!("{:?}", row);
    }
    let quadrants = into_quadrants(matrix).unwrap();
    println!("{:#?}", &quadrants);
}

fn get_quadrants(font: Font) -> std::collections::HashMap<char, Quadrants<u8>> {
    // This is the most comprehensive list of geometric characters I've found, starting with traditional ASCII characters and delving into obscure Unicode sourced from god knows where, in one long string.
    let geometric_chars = "─━│┃┄┅┆┇┈┉┊┋┌┍┎┏┐┑┒┓└┕┖┗┘┙┚┛├┝┞┟┠┡┢┣┤┥┦┧┨┩┪┫┬┭┮┯┰┱┲┳┴┵┶┷┸┹┺┻┼┽┾┿╀╁╂╃╄╅╆╇╈╉╊╋╌╍╎╏═║╒╓╔╕╖╗╘╙╚╛╜╝╞╟╠╡╢╣╤╥╦╧╨╩╪╫╬╭╮╯╰╱╲╳╴╵╶╷╸╹╺╻╼╽╾╿╱╳▀▁▂▃▄▅▆▇█▉▊▋▌▍▎▏▐░▒▓▔▕▖▗▘▙▚▛▜▝▞▟□▢▣▤▥▦▧▨▩▪▫▬▭▮▯▰▱▲△▴▵▶▷▸▹►▻▼▽▾▿◀◁◂◃◄◅◆◇◈◉◊○◌◍◎●◐◑◒◓◔◕◖◗◘◙◚◛◜◝◞◟◠◡◢◣◤◥◦◧◨◩◪◫◬◭◮◯◰◱◲◳◴◵◶◷◸◹◺◻◼◽◾◿=☰☱☲☳☴☵☶☷=";
    let blocks = "▢ _⬞▣▤▥▦▧▨▩▪▫▬▭▮▯▰▱▲△▴▵▶▷▸▹►▻▼▽▾▿◀◁◂◃◄◅◆◇◈◉◊○◌◍◎●◐◑◒◓◔◕◖".chars();
    // or for this demo, just take digits 0 to 9 and turn them into chars
    let mut codepoints = (0..10).map(|i| (i, i.to_string().chars().next().unwrap()));
    // Parse it into the font type.
    geometric_chars.chars().enumerate()
        .map(|(i, c)| {
            let (metrics, coverage) = font.rasterize(c, 2 as f32);
            // coverage is a Vec<u8>, average them
            //let average = (coverage.iter().map(|u|*u as f64).sum::<f64>() / coverage.len() as f64) as u8;
            // Group into rows.
            let rows = coverage.rows(metrics.width);
            // We're interested in the top left row, the bottom left row, the top right row, and the bottom right row.
            // Check if there are at least 2 rows and 2 columns. If there aren't, return None.
            if let Some(column) = rows.get(0) {
                if column.len() < 2 {
                    return None;
                }
            } else {
                return None;
            }

            let top_left_row = rows[0][0];
            let bottom_left_row = rows[rows.len() - 1][0];
            let top_right_row = rows[0][rows[0].len() - 1];
            let bottom_right_row = rows[rows.len() - 1][rows[0].len() - 1];
            // Average them all together
            //let average: u8 = (top_left_row + bottom_left_row + top_right_row + bottom_right_row) / 4;
            //if i % 1000 == 0 {
                //println!("{}", i);
            //}
            Some((
                c,
                Quadrants {
                    top_left: top_left_row.clone(),
                    top_right: bottom_left_row.clone(),
                    bottom_left: top_right_row.clone(),
                    bottom_right: bottom_right_row.clone(),
                },
            ))
        })
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect()
}

fn main() {
    let font = include_bytes!("../data/Consolas.ttf") as &[u8];

    let font = fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();
    let (metrics, coverage) = font.rasterize('$', 150 as f32);
    let unicode_quadrants: Vec<(char, Quadrants<u8>)> = get_quadrants(font).into_iter().collect();
    //let image_quadrants = read_png_into_quadrants("pixelui/data/gui.png").unwrap();
    //print!("{:?}", image_quadrants);
    // Let's print the character g.
    // Group into rows.
    let rows = coverage.rows(metrics.width);
    // Convert the rows into quadrants.
    let image_quadrants = into_quadrants(rows).unwrap();
    let mut counter: i32 = 1;
    //loop {
        /*let (mut w, mut h) = term_size::dimensions().unwrap();
        counter += 1;
        let div1 = div!(
            w: (w as i32 - counter) % w as i32,
            h: 40,
            vrt: center,
            hrz: center,
            unit: %,
            lum: 255
        );
        let div2 = div!(
            w: 40,
            h: (h as i32 + counter + 40) % h as i32,
            vrt: center,
            hrz: center,
            unit: px,
            lum: 60
        );
        let image_quadrants = draw_div_from_scratch(w, h, div1);
        let image_quadrants = draw_div(image_quadrants, w, h, div2);*/

        let closest_characters: Vec<Vec<char>> = image_quadrants
            .par_iter()
            .map(|image_quadrants| {
                let closest_characters = image_quadrants.par_iter().map(|image_quad| {
                    // First, check if the quadrant is just zero for everything
                    if image_quad.top_left == 0 && image_quad.top_right == 0 && image_quad.bottom_left == 0 && image_quad.bottom_right == 0 {
                        return Some(' ')
                    }
                    let uni_quads_slice = unicode_quadrants.as_slice().into_iter();
                    uni_quads_slice.min_by(|(_, q1), (_, q2)| {
                        let dist = q1.distance(image_quad);
                        let dist2 = q2.distance(image_quad);
                        dist.partial_cmp(&dist2).unwrap()
                    }).map(|(c, _)| *c)

                }).filter(|c| c.is_some()).map(|c| c.unwrap()).collect::<Vec<_>>();
                closest_characters
            }
        ).collect();
        
        let lines = closest_characters.into_iter().map(|line| line.iter().collect::<String>()).collect::<Vec<_>>().join("\n");
        print!("{}", lines);
        std::thread::sleep(std::time::Duration::from_millis(20));
   // }
    //let ascii_image = rows.iter().map(|row| {
    //row.iter().map(|&c| block_map.closest_value(*c).unwrap()).collect::<String>()
    //}).collect::<Vec<_>>().join("\n");
    //println!("{}", ascii_image);
    //println!("{:#?}", metrics);
    //println!("{:#?}", font.chars());
    //let blocks_by_darkness = vec![(255, "▓"), (170, "▒"), (85, "░"), (0, " ")];
    //let block_map: std::collections::HashMap<u8, &str> = blocks_by_darkness.into_iter().collect();
}
