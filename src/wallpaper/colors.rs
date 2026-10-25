use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use amane::{Color, Service};

// twelve hues, and one more for grays, blacks and whites
pub const BUCKETS: usize = 13;
pub const NEUTRAL: usize = 12;

// a new name, so colors found by the older, less careful method are looked at again
const CACHE: &str = ".cache/amane/wallpaper-colors-v2.tsv";

// how often the folder is checked for new wallpapers
const RECHECK: Duration = Duration::from_secs(30);

// below this average colorfulness a picture counts as gray. raise it to call more
// pictures gray, lower it to call more of them colored
const NEUTRAL_BELOW: f32 = 0.004;

// the hues are sorted into this many equal slices before the winner is chosen
const SLICES: usize = 36;
const SLICE_WIDTH: f32 = 360.0 / SLICES as f32;

/*
 * the main color of every wallpaper, found once with ImageMagick and remembered in a
 * file, so the next start doesn't look at them again. it runs on its own thread, so
 * the shell never waits for it
 */
pub struct Colors {
    // red, green and blue for each file
    found: HashMap<String, [u8; 3]>,

    // files already looked at this run, so a failure isn't retried every 30 seconds
    tried: HashSet<String>,

    // how many are still waiting to be looked at
    pending: usize,
}

impl Service for Colors {
    fn new() -> Self {
        Self {
            found: read_cache(),
            tried: HashSet::new(),
            pending: 0,
        }
    }

    fn listen() {
        loop {
            analyse();

            thread::sleep(RECHECK);
        }
    }
}

impl Colors {
    pub fn pending(&self) -> usize {
        self.pending
    }

    pub fn known(&self) -> usize {
        self.found.len()
    }

    pub fn bucket(&self, file: &str) -> Option<usize> {
        self.found.get(file).map(|rgb| bucket_of(*rgb))
    }

    // the files in color order: red to purple, then grays, then the ones not looked at yet
    pub fn sorted(&self, files: Vec<String>) -> Vec<String> {
        let mut keyed: Vec<(f32, String)> = files
            .into_iter()
            .map(|file| (self.key(&file), file))
            .collect();

        keyed.sort_by(|first, second| {
            first
                .0
                .total_cmp(&second.0)
                .then_with(|| first.1.cmp(&second.1))
        });

        keyed.into_iter().map(|(_, file)| file).collect()
    }

    fn key(&self, file: &str) -> f32 {
        let Some(rgb) = self.found.get(file) else {
            return 1000.0;
        };

        let (hue, _, lightness) = hsl(*rgb);

        let bucket = bucket_of(*rgb);

        if bucket == NEUTRAL {
            NEUTRAL as f32 * 10.0 + lightness
        } else {
            bucket as f32 * 10.0 + (hue - bucket as f32 * 30.0) / 30.0
        }
    }
}

// the color a swatch in the picker's bar shows
pub fn swatch(bucket: usize) -> Color {
    if bucket == NEUTRAL {
        return Color::rgb(136, 136, 136);
    }

    let [red, green, blue] = from_hsl(bucket as f32 * 30.0 + 15.0, 0.65, 0.5);

    Color::rgb(red, green, blue)
}

fn analyse() {
    let files = super::list_files();

    let todo: Vec<String> = {
        let colors = Colors::read();

        files
            .into_iter()
            .filter(|file| !colors.found.contains_key(file) && !colors.tried.contains(file))
            .collect()
    };

    if todo.is_empty() {
        return;
    }

    Colors::write().pending = todo.len();

    for file in todo {
        let rgb = look(&file);

        let mut colors = Colors::write();

        colors.tried.insert(file.clone());

        if let Some(rgb) = rgb {
            colors.found.insert(file, rgb);
        }

        colors.pending = colors.pending.saturating_sub(1);
    }

    save_cache();
}

/*
 * ImageMagick shrinks the picture, boils it down to 32 colors and prints how many
 * pixels each has. summarise() then decides what the picture's color is
 */
fn look(path: &str) -> Option<[u8; 3]> {
    let command = format!(
        "magick -define jpeg:size=200x200 {} -resize 96x96! +dither -colors 32 -depth 8 -format %c histogram:info:-",
        super::quote(path)
    );

    let text = amane::output(&command);

    let mut entries: Vec<(f32, [u8; 3])> = Vec::new();

    // lines look like "  1234: (  0, 51,102) #003366 srgb(0,51,102)"
    for line in text.lines() {
        let Some((count, rest)) = line.split_once(':') else {
            continue;
        };

        let Ok(count) = count.trim().parse::<f32>() else {
            continue;
        };

        let Some(hash) = rest.find('#') else {
            continue;
        };

        let Some(hex) = rest.get(hash + 1..hash + 7) else {
            continue;
        };

        let Ok(value) = u32::from_str_radix(hex, 16) else {
            continue;
        };

        entries.push((count, [(value >> 16) as u8, (value >> 8) as u8, value as u8]));
    }

    summarise(&entries)
}

/*
 * every color votes for its hue with a weight of its pixel count times how colorful
 * it is, squared, so a vivid color with few pixels outweighs a dull one with many.
 * the votes are added up in hue slices, and the slice with the most (counting a half
 * share of its neighbors) wins. only a picture with almost no color anywhere is called gray
 */
fn summarise(entries: &[(f32, [u8; 3])]) -> Option<[u8; 3]> {
    let total: f32 = entries.iter().map(|(count, _)| count).sum();

    if total <= 0.0 {
        return None;
    }

    let mut slices = [0.0_f32; SLICES];
    let mut colorful = 0.0;
    let mut lightness_total = 0.0;

    for (count, rgb) in entries {
        let (hue, _, lightness) = hsl(*rgb);

        let chroma = chroma_of(*rgb);

        // nearly black and nearly white colors say little about the picture's color
        let extreme = (2.0 * lightness - 1.0).abs().powi(3);

        let weight = count * chroma * chroma * (1.0 - extreme);

        colorful += weight;
        lightness_total += count * lightness;

        let slice = ((hue / SLICE_WIDTH) as usize).min(SLICES - 1);

        slices[slice] += weight;
    }

    if colorful / total < NEUTRAL_BELOW {
        let gray = (lightness_total / total * 255.0).round() as u8;

        return Some([gray, gray, gray]);
    }

    let mut best = 0;
    let mut best_value = 0.0;

    for index in 0..SLICES {
        let before = slices[(index + SLICES - 1) % SLICES];
        let after = slices[(index + 1) % SLICES];

        let value = before * 0.5 + slices[index] + after * 0.5;

        if value > best_value {
            best = index;
            best_value = value;
        }
    }

    // the exact hue: the average of the winning slice and its two neighbors
    let mut weighted = 0.0;
    let mut sum = 0.0;

    for step in [-1_i32, 0, 1] {
        let slice = (best as i32 + step).rem_euclid(SLICES as i32) as usize;

        let weight = slices[slice];

        weighted += weight * (best as f32 + step as f32 + 0.5) * SLICE_WIDTH;
        sum += weight;
    }

    let hue = if sum > 0.0 {
        (weighted / sum).rem_euclid(360.0)
    } else {
        (best as f32 + 0.5) * SLICE_WIDTH
    };

    Some(from_hsl(hue, 0.65, 0.5))
}

fn bucket_of(rgb: [u8; 3]) -> usize {
    let (hue, saturation, lightness) = hsl(rgb);

    if saturation < 0.18 || lightness < 0.10 || lightness > 0.92 {
        return NEUTRAL;
    }

    ((hue / 30.0) as usize).min(11)
}

// how far the strongest of red, green and blue is from the weakest, from 0 to 1
fn chroma_of(rgb: [u8; 3]) -> f32 {
    let highest = rgb[0].max(rgb[1]).max(rgb[2]);
    let lowest = rgb[0].min(rgb[1]).min(rgb[2]);

    f32::from(highest - lowest) / 255.0
}

// hue in degrees, then saturation and lightness from 0 to 1
fn hsl(rgb: [u8; 3]) -> (f32, f32, f32) {
    let red = f32::from(rgb[0]) / 255.0;
    let green = f32::from(rgb[1]) / 255.0;
    let blue = f32::from(rgb[2]) / 255.0;

    let highest = red.max(green).max(blue);
    let lowest = red.min(green).min(blue);

    let lightness = (highest + lowest) / 2.0;
    let spread = highest - lowest;

    if spread == 0.0 {
        return (0.0, 0.0, lightness);
    }

    let saturation = spread / (1.0 - (2.0 * lightness - 1.0).abs());

    let sector = if highest == red {
        ((green - blue) / spread).rem_euclid(6.0)
    } else if highest == green {
        (blue - red) / spread + 2.0
    } else {
        (red - green) / spread + 4.0
    };

    (sector * 60.0, saturation, lightness)
}

fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> [u8; 3] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;

    let x = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());

    let (red, green, blue) = match (hue / 60.0) as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    let shift = lightness - chroma / 2.0;

    [
        ((red + shift) * 255.0).round() as u8,
        ((green + shift) * 255.0).round() as u8,
        ((blue + shift) * 255.0).round() as u8,
    ]
}

fn cache_file() -> String {
    format!("{}/{CACHE}", std::env::var("HOME").unwrap_or_default())
}

// each line is a path, a tab, and a color like 1a2b3c
fn read_cache() -> HashMap<String, [u8; 3]> {
    let text = fs::read_to_string(cache_file()).unwrap_or_default();

    let mut found = HashMap::new();

    for line in text.lines() {
        let Some((path, hex)) = line.split_once('\t') else {
            continue;
        };

        let Ok(value) = u32::from_str_radix(hex, 16) else {
            continue;
        };

        found.insert(
            String::from(path),
            [(value >> 16) as u8, (value >> 8) as u8, value as u8],
        );
    }

    found
}

fn save_cache() {
    let mut text = String::new();

    {
        let colors = Colors::read();

        for (path, rgb) in &colors.found {
            text.push_str(&format!("{path}\t{:02x}{:02x}{:02x}\n", rgb[0], rgb[1], rgb[2]));
        }
    }

    let file = cache_file();

    if let Some(folder) = Path::new(&file).parent() {
        let _ = fs::create_dir_all(folder);
    }

    // a failed save only means the colors are looked at again next time
    let _ = fs::write(file, text);
}
