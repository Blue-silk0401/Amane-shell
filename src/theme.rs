use amane::{Color, Palette, Service};

/*
 * The whole shell's colors, built from the wallpaper. Hues come from the picture;
 * lightness is fixed per role, so the theme stays dark and readable on any image.
 */

fn to_hsl(color: Color) -> (f32, f32, f32) {
    let r = color.red() as f32 / 255.0;
    let g = color.green() as f32 / 255.0;
    let b = color.blue() as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let spread = max - min;

    if spread < 1e-6 {
        return (0.0, 0.0, lightness);
    }

    let saturation = spread / (1.0 - (2.0 * lightness - 1.0).abs());

    let hue = if max == r {
        ((g - b) / spread).rem_euclid(6.0)
    } else if max == g {
        (b - r) / spread + 2.0
    } else {
        (r - g) / spread + 4.0
    };

    (hue * 60.0, saturation, lightness)
}

fn from_hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
    let hue = hue.rem_euclid(360.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let x = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = lightness - chroma / 2.0;

    let (r, g, b) = match ((hue / 60.0) as u32) % 6 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    let byte = |value: f32| ((value + m) * 255.0).round().clamp(0.0, 255.0) as u8;

    Color::rgb(byte(r), byte(g), byte(b))
}

const SECTORS: usize = 12;

/*
 * The picture's main color as (hue, saturation). The palette lists colors most
 * common first, so each colorful one adds a vote that shrinks with its rank, and
 * neighbouring hues share votes: many shades of blue beat one small red patch.
 */
fn main_color() -> (f32, f32) {
    let palette = Palette::read();

    let mut votes = [0.0_f32; SECTORS];
    let mut strongest = [(0.0_f32, 0.0_f32); SECTORS];

    for (rank, &color) in palette.colors().iter().enumerate() {
        let (hue, saturation, lightness) = to_hsl(color);

        // greys, near black and near white say nothing about the picture's color
        if saturation < 0.20 || !(0.08..=0.92).contains(&lightness) {
            continue;
        }

        let sector = ((hue / 30.0) as usize) % SECTORS;

        votes[sector] += (0.5 + saturation) / (rank as f32 + 1.0);

        if saturation > strongest[sector].1 {
            strongest[sector] = (hue, saturation);
        }
    }

    let mut winner = 0;
    let mut winning_score = 0.0;

    for sector in 0..SECTORS {
        let before = votes[(sector + SECTORS - 1) % SECTORS];
        let after = votes[(sector + 1) % SECTORS];

        let score = votes[sector] + 0.5 * (before + after);

        if score > winning_score {
            winner = sector;
            winning_score = score;
        }
    }

    // the strongest colour among the winner and its two neighbours
    let mut chosen = (0.0_f32, 0.0_f32);

    for offset in [SECTORS - 1, 0, 1] {
        let candidate = strongest[(winner + offset) % SECTORS];

        if candidate.1 > chosen.1 {
            chosen = candidate;
        }
    }

    // a grey picture has no colourful palette entry, so use whatever tint it has
    if chosen.1 == 0.0 {
        let (hue, saturation, _) = to_hsl(palette.dominant());

        return (hue, saturation);
    }

    chosen
}

// a dark base carrying a little of the picture's color
fn base(lightness: f32) -> Color {
    let (hue, saturation) = main_color();

    from_hsl(hue, (saturation * 0.6).clamp(0.08, 0.40), lightness)
}

pub fn bg() -> Color {
    base(0.10)
}

pub fn surface() -> Color {
    base(0.17)
}

pub fn hover() -> Color {
    base(0.24)
}

pub fn text() -> Color {
    let (hue, _) = main_color();

    from_hsl(hue, 0.30, 0.89)
}

pub fn muted() -> Color {
    let (hue, _) = main_color();

    from_hsl(hue, 0.15, 0.55)
}

pub fn accent() -> Color {
    let (hue, saturation) = main_color();

    from_hsl(hue, saturation.clamp(0.50, 0.90), 0.68)
}

pub fn outline() -> Color {
    accent()
}

pub fn on_accent() -> Color {
    bg()
}

// these keep their meaning whatever the wallpaper is
pub fn success() -> Color {
    Color::rgb(0xa6, 0xe3, 0xa1)
}

pub fn danger() -> Color {
    Color::rgb(0xf3, 0x8b, 0xa8)
}

// a cava theme file: a gradient from the picture's main color, rising in brightness
pub fn cava_theme() -> String {
    let (hue, saturation) = main_color();
    let saturation = saturation.clamp(0.60, 0.90);

    let mut text = String::from("[color]\ngradient = 1\n");

    for step in 0..6 {
        let progress = step as f32 / 5.0;
        let color = from_hsl(hue + progress * 40.0, saturation, 0.42 + progress * 0.36);

        text.push_str(&format!("gradient_color_{} = '{}'\n", step + 1, color.hex()));
    }

    text
}

// ghostty colors: background, text and cursor follow the picture, the ANSI hues stay recognisable
pub fn ghostty_theme() -> String {
    let (hue, _) = main_color();

    // rotate the whole scheme toward the picture, by 60% of the way from a pink reference
    let shift = ((hue - 330.0 + 540.0) % 360.0 - 180.0) * 0.25;

    // red, green, yellow, blue, magenta, cyan
    let mut hues = [345.0, 125.0, 40.0, 225.0, 305.0, 185.0];

    // the "blue" slot carries the picture's own hue, so prompts that use blue match the theme
    hues[3] = hue - shift;

    let mut text = String::new();

    text.push_str(&format!("background = {}\n", bg().hex()));
    text.push_str(&format!("foreground = {}\n", self::text().hex()));
    text.push_str(&format!("cursor-color = {}\n", accent().hex()));
    text.push_str(&format!("selection-background = {}\n", hover().hex()));
    text.push_str(&format!("selection-foreground = {}\n", self::text().hex()));

    text.push_str(&format!("palette = 0={}\n", bg().hex()));

    for (index, base_hue) in hues.iter().enumerate() {
        text.push_str(&format!("palette = {}={}\n", index + 1, from_hsl(*base_hue + shift, 0.50, 0.62).hex()));
    }

    text.push_str(&format!("palette = 7={}\n", from_hsl(hue, 0.20, 0.82).hex()));
    text.push_str(&format!("palette = 8={}\n", base(0.20).hex()));

    for (index, base_hue) in hues.iter().enumerate() {
        text.push_str(&format!("palette = {}={}\n", index + 9, from_hsl(*base_hue + shift, 0.60, 0.72).hex()));
    }

    text.push_str(&format!("palette = 15={}\n", from_hsl(hue, 0.25, 0.94).hex()));

    text
}

// niri border colors, merged into the main config through an include
pub fn niri_theme() -> String {
    let (hue, saturation) = main_color();
    let inactive = from_hsl(hue, (saturation * 0.4).clamp(0.10, 0.30), 0.32);

    format!(
        "layout {{\n    border {{\n        active-color \"{}\"\n        inactive-color \"{}\"\n        urgent-color \"{}\"\n    }}\n}}\n",
        accent().hex(),
        inactive.hex(),
        danger().hex(),
    )
}

// three colors rising in brightness around a hue, for btop's gradients
fn ramp(hue: f32, saturation: f32) -> [Color; 3] {
    [
        from_hsl(hue - 25.0, saturation, 0.50),
        from_hsl(hue, saturation, 0.62),
        from_hsl(hue + 25.0, saturation, 0.74),
    ]
}

// a btop theme file: boxes, text and graphs follow the picture
pub fn btop_theme() -> String {
    let (hue, saturation) = main_color();
    let strength = saturation.clamp(0.50, 0.85);

    let mut text = String::new();

    let mut set = |key: &str, color: Color| {
        text.push_str(&format!("theme[{key}]=\"{}\"\n", color.hex()));
    };

    set("main_bg", bg());
    set("main_fg", self::text());
    set("title", self::text());
    set("hi_fg", accent());
    set("selected_bg", hover());
    set("selected_fg", self::text());
    set("inactive_fg", muted());
    set("graph_text", muted());
    set("meter_bg", surface());
    set("proc_misc", accent());

    let box_color = from_hsl(hue, 0.40, 0.45);

    for name in ["cpu_box", "mem_box", "net_box", "proc_box"] {
        set(name, box_color);
    }

    set("div_line", base(0.22));

    // temperature keeps its meaning: green, yellow, red
    set("temp_start", success());
    set("temp_mid", Color::rgb(0xf9, 0xe2, 0xaf));
    set("temp_end", danger());

    let groups = [
        ("cpu", hue),
        ("free", hue),
        ("cached", hue + 40.0),
        ("available", hue - 40.0),
        ("used", hue + 80.0),
        ("download", hue - 30.0),
        ("upload", hue + 30.0),
        ("process", hue),
    ];

    for (name, group_hue) in groups {
        let colors = ramp(group_hue, strength);

        set(&format!("{name}_start"), colors[0]);
        set(&format!("{name}_mid"), colors[1]);
        set(&format!("{name}_end"), colors[2]);
    }

    text
}

// the six ANSI-style colors (red, green, yellow, blue, magenta, cyan), shifted toward the picture like ghostty's
fn ansi_colors() -> [Color; 6] {
    let (hue, _) = main_color();

    let shift = ((hue - 330.0 + 540.0) % 360.0 - 180.0) * 0.25;
    let hues = [345.0, 125.0, 40.0, hue - shift, 305.0, 185.0];

    hues.map(|base_hue| from_hsl(base_hue + shift, 0.50, 0.66))
}

// a lua table of colors, read by the neovim snippet in the nvf config
pub fn nvim_theme() -> String {
    let [red, green, yellow, blue, magenta, cyan] = ansi_colors();

    format!(
        "return {{\n  surface = \"{}\",\n  hover = \"{}\",\n  text = \"{}\",\n  muted = \"{}\",\n  accent = \"{}\",\n  red = \"{}\",\n  green = \"{}\",\n  yellow = \"{}\",\n  blue = \"{}\",\n  magenta = \"{}\",\n  cyan = \"{}\",\n}}\n",
        surface().hex(),
        hover().hex(),
        text().hex(),
        muted().hex(),
        accent().hex(),
        red.hex(),
        green.hex(),
        yellow.hex(),
        blue.hex(),
        magenta.hex(),
        cyan.hex(),
    )
}

// gtk css: the accent color for libadwaita (gtk 4) and the selection color for gtk 3
pub fn gtk_theme() -> String {
    let accent = accent().hex();
    let on_accent = bg().hex();

    format!(
        "/* written by amane from the wallpaper */\n@define-color accent_color {accent};\n@define-color accent_bg_color {accent};\n@define-color accent_fg_color {on_accent};\n@define-color theme_selected_bg_color {accent};\n@define-color theme_selected_fg_color {on_accent};\n"
    )
}
