use anyhow::{Result, bail};
use clap::ValueEnum;
use regex::Regex;
use std::io::Write;
use std::{fmt::Display, sync::LazyLock};

static HEX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3}|[A-Fa-f0-9]{4}|[A-Fa-f0-9]{8})$").unwrap()
});

static RGB_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^rgb\s*\(\s*([.\d]+)[, ]+([.\d]+)[, ]+([.\d]+)\s*\)\s*$").unwrap()
});

static RGBA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^rgba\s*\(\s*([.\d]+)[, ]+([.\d]+)[, ]+([.\d]+)[, ]+([.\d]+%?)\s*\)\s*$")
        .unwrap()
});

static OKLAB_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^oklab\s*\(\s*(-?[.\d]+%?)[, ]+(-?[.\d]+%?)[, ]+(-?[.\d]+%?)\s*(?:[/, ]\s*([.\d]+%?)\s*)?\)\s*$").unwrap()
});

static OKLCH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^oklch\s*\(\s*([.\d]+%?)[, ]+([.\d]+%?)[, ]+(-?[.\d]+(?:deg)?%?)\s*(?:[/, ]\s*([.\d]+%?)\s*)?\)\s*$").unwrap()
});

#[derive(Debug, PartialEq, Eq, Clone, ValueEnum)]
pub enum Color {
    Hex,
    Rgb,
    Rgba,
    Oklab,
    Oklch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    alpha: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Oklab {
    l: f64,
    a: f64,
    b: f64,
    alpha: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Oklch {
    l: f64,
    c: f64,
    h: f64,
    alpha: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Xyz {
    x: f64,
    y: f64,
    z: f64,
}

trait ToXyz {
    fn to_xyz(&self) -> Xyz;
}

trait FromXyz {
    fn from_xyz(xyz: &Xyz, alpha: f64) -> Self;
}

fn identify_color(color: &str) -> Result<Color> {
    if HEX_REGEX.is_match(color) {
        Ok(Color::Hex)
    } else if RGB_REGEX.is_match(color) {
        Ok(Color::Rgb)
    } else if RGBA_REGEX.is_match(color) {
        Ok(Color::Rgba)
    } else if OKLAB_REGEX.is_match(color) {
        Ok(Color::Oklab)
    } else if OKLCH_REGEX.is_match(color) {
        Ok(Color::Oklch)
    } else {
        bail!("invalid/unsupported color: {color}")
    }
}

fn parse_hex(hex: &str) -> Result<Rgba> {
    let hex = hex.trim_start_matches('#');

    let expanded = match hex.len() {
        3 => {
            let chars: Vec<char> = hex.chars().collect();
            format!("{0}{0}{1}{1}{2}{2}ff", chars[0], chars[1], chars[2])
        }
        4 => {
            let chars: Vec<char> = hex.chars().collect();
            format!(
                "{0}{0}{1}{1}{2}{2}{3}{3}",
                chars[0], chars[1], chars[2], chars[3]
            )
        }
        6 => format!("{hex}ff"),
        8 => hex.to_string(),
        _ => bail!("invalid hex color value: #{hex}"),
    };

    let r = u8::from_str_radix(&expanded[0..2], 16)?;
    let g = u8::from_str_radix(&expanded[2..4], 16)?;
    let b = u8::from_str_radix(&expanded[4..6], 16)?;
    let alpha = u8::from_str_radix(&expanded[6..8], 16)? as f64 / 255.0;

    Ok(Rgba { r, g, b, alpha })
}

fn parse_rgba(color: &str) -> Result<Rgba> {
    let parts = extract_color_parts(color, &['r', 'g', 'b', 'a', '(', ')', ' '])?;

    if parts.len() != 3 && parts.len() != 4 {
        bail!("invalid rgb/rgba value: {color}");
    }

    let r = parse_unit_aware(&parts[0], 255.0, |v| v.round() as u8)?;
    let g = parse_unit_aware(&parts[1], 255.0, |v| v.round() as u8)?;
    let b = parse_unit_aware(&parts[2], 255.0, |v| v.round() as u8)?;
    let alpha = parts.get(3).map_or(Ok(1.0), |a| parse_alpha(a))?;

    Ok(Rgba { r, g, b, alpha })
}

fn parse_oklab(color: &str) -> Result<Oklab> {
    let parts = extract_color_parts(color, &['o', 'k', 'l', 'a', 'b', '(', ')', ' '])?;

    if parts.len() != 3 && parts.len() != 4 {
        bail!("invalid oklab value: {color}");
    }

    let l = parse_unit_aware(&parts[0], 1.0, |v| v)?;
    let a = parse_unit_aware(&parts[1], 1.0, |v| v)?;
    let b = parse_unit_aware(&parts[2], 1.0, |v| v)?;
    let alpha = parts.get(3).map_or(Ok(1.0), |a| parse_alpha(a))?;

    Ok(Oklab { l, a, b, alpha })
}

fn parse_oklch(color: &str) -> Result<Oklch> {
    let parts = extract_color_parts(color, &['o', 'k', 'l', 'c', 'h', '(', ')', ' '])?;

    if parts.len() != 3 && parts.len() != 4 {
        bail!("invalid oklch value: {color}");
    }

    let l = parse_unit_aware(&parts[0], 1.0, |v| v)?;
    let c = parse_unit_aware(&parts[1], 1.0, |v| v)?;
    let h = parse_unit_aware(&parts[2], 1.0, |v| v)?;
    let alpha = parts.get(3).map_or(Ok(1.0), |a| parse_alpha(a))?;

    Ok(Oklch { l, c, h, alpha })
}

fn extract_color_parts(s: &str, matches: &[char]) -> Result<Vec<String>> {
    let cleaned = s
        .to_lowercase()
        .trim_matches(matches)
        .replace(",", " ")
        .replace("/", " ");
    let parts = cleaned.split_whitespace().map(String::from).collect();
    Ok(parts)
}

/// max will only be used for percentage values for other units max will be ignored
fn parse_unit_aware<T>(s: &str, max: f64, convert: impl Fn(f64) -> T) -> Result<T> {
    let value = if let Some(stripped) = s.strip_suffix("%") {
        let percentage = stripped.parse::<f64>().map_err(anyhow::Error::from)?;
        percentage / 100.0 * max
    } else if let Some(stripped) = s.strip_suffix("deg") {
        stripped.parse::<f64>().map_err(anyhow::Error::from)?
    } else {
        s.parse::<f64>().map_err(anyhow::Error::from)?
    };

    Ok(convert(value))
}

fn parse_alpha(s: &str) -> Result<f64> {
    let alpha = parse_unit_aware(s, 1.0, |v| v)?;
    if alpha < 0.0 || alpha > 1.0 {
        bail!("alpha value out of range. expected value between 0 and 1");
    }
    Ok(alpha)
}

fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f64) -> f64 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

impl ToXyz for Rgba {
    fn to_xyz(&self) -> Xyz {
        let r = srgb_to_linear(self.r as f64 / 255.0);
        let g = srgb_to_linear(self.g as f64 / 255.0);
        let b = srgb_to_linear(self.b as f64 / 255.0);

        let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
        let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
        let z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b;

        Xyz { x, y, z }
    }
}

impl FromXyz for Rgba {
    fn from_xyz(xyz: &Xyz, alpha: f64) -> Self {
        let r = 3.2404542 * xyz.x - 1.5371385 * xyz.y - 0.4985314 * xyz.z;
        let g = -0.9692660 * xyz.x + 1.8760108 * xyz.y + 0.0415560 * xyz.z;
        let b = 0.0556434 * xyz.x - 0.2040259 * xyz.y + 1.0572252 * xyz.z;

        let scale = |v: f64| -> u8 { (linear_to_srgb(v) * 255.0).round().clamp(0.0, 255.0) as u8 };

        Rgba {
            r: scale(r),
            g: scale(g),
            b: scale(b),
            alpha,
        }
    }
}

impl ToXyz for Oklab {
    fn to_xyz(&self) -> Xyz {
        let l = self.l + 0.3963377774 * self.a + 0.2158037573 * self.b;
        let m = self.l - 0.1055613458 * self.a - 0.0638541728 * self.b;
        let s = self.l - 0.0894841775 * self.a - 1.2914855480 * self.b;

        let l_ = l.powi(3);
        let m_ = m.powi(3);
        let s_ = s.powi(3);

        Xyz {
            x: 1.2270138511 * l_ - 0.5577999807 * m_ + 0.2812561490 * s_,
            y: -0.0405801784 * l_ + 1.1122568696 * m_ - 0.0716766787 * s_,
            z: -0.0763812845 * l_ - 0.4214819784 * m_ + 1.5861632204 * s_,
        }
    }
}

impl FromXyz for Oklab {
    fn from_xyz(xyz: &Xyz, alpha: f64) -> Self {
        let l = 0.8190224432 * xyz.x + 0.3619062563 * xyz.y - 0.1288737826 * xyz.z;
        let m = 0.0329836672 * xyz.x + 0.9292868469 * xyz.y + 0.0361446682 * xyz.z;
        let s = 0.0481771996 * xyz.x + 0.2642395249 * xyz.y + 0.6335478258 * xyz.z;

        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();

        Oklab {
            l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
            alpha,
        }
    }
}

impl Oklch {
    fn to_oklab(&self) -> Oklab {
        Oklab {
            l: self.l,
            a: self.c * self.h.to_radians().cos(),
            b: self.c * self.h.to_radians().sin(),
            alpha: self.alpha,
        }
    }
}

impl ToXyz for Oklch {
    fn to_xyz(&self) -> Xyz {
        self.to_oklab().to_xyz()
    }
}

impl Oklab {
    fn to_oklch(&self) -> Oklch {
        let c = (self.a * self.a + self.b * self.b).sqrt();
        let mut h = self.b.atan2(self.a).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        Oklch {
            l: self.l,
            c,
            h,
            alpha: self.alpha,
        }
    }
}

impl FromXyz for Oklch {
    fn from_xyz(xyz: &Xyz, alpha: f64) -> Self {
        Oklab::from_xyz(xyz, alpha).to_oklch()
    }
}

impl Display for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rgba({}, {}, {}, {})",
            self.r, self.g, self.b, self.alpha
        )
    }
}

impl Display for Oklab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.alpha == 1.0 {
            write!(f, "oklab({} {} {})", self.l, self.a, self.b)
        } else {
            write!(
                f,
                "oklab({} {} {} / {})",
                self.l, self.a, self.b, self.alpha
            )
        }
    }
}

impl Display for Oklch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.alpha == 1.0 {
            write!(f, "oklch({} {} {})", self.l, self.c, self.h)
        } else {
            write!(
                f,
                "oklch({} {} {} / {})",
                self.l, self.c, self.h, self.alpha
            )
        }
    }
}

fn rgba_to_hex_string(rgba: &Rgba) -> String {
    if rgba.alpha == 1.0 {
        format!("#{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b)
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            rgba.r,
            rgba.g,
            rgba.b,
            (rgba.alpha * 255.0).round().clamp(0.0, 255.0) as u8
        )
    }
}
fn rgba_to_rgb_string(rgba: &Rgba) -> String {
    format!("rgb({}, {}, {})", rgba.r, rgba.g, rgba.b)
}

fn convert(color: &str, target: &Color) -> Result<String> {
    let source = identify_color(color)?;

    let (xyz, alpha) = match source {
        Color::Hex | Color::Rgb | Color::Rgba => {
            let rgba = if source == Color::Hex {
                parse_hex(color)?
            } else {
                parse_rgba(color)?
            };

            (rgba.to_xyz(), rgba.alpha)
        }
        Color::Oklab => {
            let oklab = parse_oklab(color)?;
            (oklab.to_xyz(), oklab.alpha)
        }
        Color::Oklch => {
            let oklch = parse_oklch(color)?;
            (oklch.to_xyz(), oklch.alpha)
        }
    };

    match target {
        Color::Hex | Color::Rgb | Color::Rgba => {
            let rgba = Rgba::from_xyz(&xyz, alpha);
            if target == &Color::Hex {
                Ok(rgba_to_hex_string(&rgba))
            } else if target == &Color::Rgb {
                Ok(rgba_to_rgb_string(&rgba))
            } else {
                Ok(format!("{}", rgba))
            }
        }
        Color::Oklab => Ok(format!("{}", Oklab::from_xyz(&xyz, alpha))),
        Color::Oklch => Ok(format!("{}", Oklch::from_xyz(&xyz, alpha))),
    }
}

pub fn convert_colors(colors: &[String], target: &Color) -> Vec<Result<(String, String)>> {
    colors
        .iter()
        .map(|color| convert(color, target).map(|converted| (color.to_string(), converted)))
        .collect()
}

pub fn log_conversion_results(
    results: &[Result<(String, String)>],
    output_only: bool,
    w_writer: &mut impl Write,
    l_writer: &mut impl Write,
) {
    for r in results {
        match r {
            Ok((original, converted)) => {
                if output_only {
                    let _ = writeln!(w_writer, "{converted}");
                } else {
                    let _ = writeln!(w_writer, "{original} -> {converted}");
                }
            }
            Err(e) => {
                let _ = writeln!(l_writer, "error: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase<In, Out> {
        input: In,
        output: Out,
    }

    #[test]
    fn test_hex_identification() -> Result<()> {
        let three_hex = String::from("#fff");
        let four_hex = String::from("#ffff");
        let six_hex = String::from("#f0f0ff");
        let eight_hex = String::from("#f3ffffff");

        assert_eq!(Color::Hex, identify_color(&three_hex)?);
        assert_eq!(Color::Hex, identify_color(&four_hex)?);
        assert_eq!(Color::Hex, identify_color(&six_hex)?);
        assert_eq!(Color::Hex, identify_color(&eight_hex)?);

        Ok(())
    }

    #[test]
    fn test_rgb_identification() -> Result<()> {
        let rgb_1 = String::from("rgb(1, 0, 255)");
        let rgb_2 = String::from("rgb (1 0 255)  ");

        assert_eq!(Color::Rgb, identify_color(&rgb_1)?);
        assert_eq!(Color::Rgb, identify_color(&rgb_2)?);

        Ok(())
    }

    #[test]
    fn test_rgba_identification() -> Result<()> {
        let rgb_1 = String::from("rgba(1, 0,255,0.1)");
        let rgb_2 = String::from("rgba (1 3 255 0.1)  ");
        let rgb_3 = String::from("rgba (1 3, 255, 0.1)  ");

        assert_eq!(Color::Rgba, identify_color(&rgb_1)?);
        assert_eq!(Color::Rgba, identify_color(&rgb_2)?);
        assert_eq!(Color::Rgba, identify_color(&rgb_3)?);

        Ok(())
    }

    #[test]
    fn test_oklab_identification() -> Result<()> {
        let space_sep = String::from("oklab(0.5 0.2 0.1)");
        let comma_sep = String::from("oklab(0.5, 0.2, 0.1)");
        let with_alpha = String::from("oklab(0.5 0.2 0.1 / 0.5)");
        let comma_alpha = String::from("oklab(0.5, 0.2, 0.1, 1)");
        let with_percent = String::from("oklab(50% 0.2 0.1% / 25%)");
        let mixed_sep = String::from("oklab(0.5, 0.2 0.1)");
        let space_separated_alpha = String::from("oklab(0.5, 0.2 0.1 0.9)");

        assert_eq!(Color::Oklab, identify_color(&space_sep)?);
        assert_eq!(Color::Oklab, identify_color(&comma_sep)?);
        assert_eq!(Color::Oklab, identify_color(&with_alpha)?);
        assert_eq!(Color::Oklab, identify_color(&comma_alpha)?);
        assert_eq!(Color::Oklab, identify_color(&with_percent)?);
        assert_eq!(Color::Oklab, identify_color(&mixed_sep)?);
        assert_eq!(Color::Oklab, identify_color(&space_separated_alpha)?);

        Ok(())
    }

    #[test]
    fn test_oklch_identification() -> Result<()> {
        let space_sep = String::from("oklch(0.5 0.2 300)");
        let with_hue_deg = String::from("oklch(0.5 0.2 30deg)");
        let with_alpha = String::from("oklch(0.5 0.2 300 / 0.5)");
        let comma_alpha = String::from("oklch(0.5, 0.2, 300, 1)");
        let with_percent = String::from("oklch(50% 0.2 300deg / 25%)");

        assert_eq!(Color::Oklch, identify_color(&space_sep)?);
        assert_eq!(Color::Oklch, identify_color(&with_hue_deg)?);
        assert_eq!(Color::Oklch, identify_color(&with_alpha)?);
        assert_eq!(Color::Oklch, identify_color(&comma_alpha)?);
        assert_eq!(Color::Oklch, identify_color(&with_percent)?);

        Ok(())
    }

    #[test]
    fn test_case_insensitive_prefixes() -> Result<()> {
        assert_eq!(Color::Hex, identify_color("#FFE4E1")?);
        assert_eq!(Color::Rgb, identify_color("RGB(1, 0, 255)")?);
        assert_eq!(Color::Rgba, identify_color("RGBA(1, 0, 255, 0.1)")?);
        assert_eq!(Color::Oklab, identify_color("OKLAB(0.5 0.2 0.1)")?);
        assert_eq!(Color::Oklch, identify_color("OKLCH(0.5 0.2 300)")?);

        Ok(())
    }

    #[test]
    fn test_decimal_rgb_components() -> Result<()> {
        assert_eq!(Color::Rgb, identify_color("rgb(1.5, 0, 255.25)")?);
        assert_eq!(Color::Rgba, identify_color("rgba(.5, 1, 2.5, 0.1)")?);

        Ok(())
    }

    #[test]
    fn test_unsupported_or_malformed_colors() {
        let invalid = [
            "",
            "   ",
            "#",
            "#12",
            "#12345",
            "#1234567",
            "#ggg",
            "fff",
            "  #fff  ",
            "rgb()",
            "rgb(255)",
            "rgb(255, 255)",
            "rgba(1, 0, 255)",
            "rgba(1, 0, 255,)",
            "oklab()",
            "oklab(1 2)",
            "oklab(1 2 3 /)",
            "hsl(120, 50%, 50%)",
            "hsla(120, 50%, 50%, 1)",
            "not a color",
        ];

        for input in invalid {
            assert!(
                identify_color(input).is_err(),
                "expected an error for: {input}"
            );
        }
    }

    #[test]
    fn test_hex_parsing() -> Result<()> {
        let test_cases: Vec<TestCase<String, Rgba>> = vec![
            TestCase {
                input: "#fff".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#ffff".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#ffffff".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#ffffffff".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#000".to_owned(),
                output: Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#f00".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#f0f".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#f0ff".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#0f0f".to_owned(),
                output: Rgba {
                    r: 0,
                    g: 255,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#80ff00".to_owned(),
                output: Rgba {
                    r: 128,
                    g: 255,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#ff000080".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 0.5019607843137255,
                },
            },
            TestCase {
                input: "#aBc".to_owned(),
                output: Rgba {
                    r: 170,
                    g: 187,
                    b: 204,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "#1234".to_owned(),
                output: Rgba {
                    r: 17,
                    g: 34,
                    b: 51,
                    alpha: 0.26666666666666666,
                },
            },
        ];

        for h in test_cases {
            assert_eq!(parse_hex(&h.input)?, h.output);
        }

        Ok(())
    }

    #[test]
    fn test_rgb_parsing() -> Result<()> {
        let test_cases: Vec<TestCase<String, Rgba>> = vec![
            TestCase {
                input: "rgb(255, 0, 0)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "rgb(1 0 255)".to_owned(),
                output: Rgba {
                    r: 1,
                    g: 0,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "rgb(0, 0, 0)".to_owned(),
                output: Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "rgba(255, 0, 0, 0.5)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "rgba(255 0 0 0.5)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "rgba(1 3 255 0.5)".to_owned(),
                output: Rgba {
                    r: 1,
                    g: 3,
                    b: 255,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "rgba(255,255,255,0)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 0.0,
                },
            },
            TestCase {
                input: "rgb (255, 0, 0)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "rgb  (1 0 255)  ".to_owned(),
                output: Rgba {
                    r: 1,
                    g: 0,
                    b: 255,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "rgba (255, 0, 0, 0.5)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "rgba  (1 3, 255, 0.5)  ".to_owned(),
                output: Rgba {
                    r: 1,
                    g: 3,
                    b: 255,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "rgba (255 255 255 0)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 0.0,
                },
            },
        ];

        for h in test_cases {
            assert_eq!(parse_rgba(&h.input)?, h.output);
        }

        Ok(())
    }

    #[test]
    fn test_parse_alpha() -> Result<()> {
        assert_eq!(parse_alpha("0")?, 0.0);
        assert_eq!(parse_alpha("0.0")?, 0.0);
        assert_eq!(parse_alpha("0.5")?, 0.5);
        assert_eq!(parse_alpha("1")?, 1.0);
        assert_eq!(parse_alpha("1.0")?, 1.0);
        assert_eq!(parse_alpha("50%")?, 0.5);
        assert_eq!(parse_alpha("100%")?, 1.0);

        assert!(parse_alpha("-0.1").is_err());
        assert!(parse_alpha("1.1").is_err());
        assert!(parse_alpha("101%").is_err());
        assert!(parse_alpha("200%").is_err());

        Ok(())
    }

    #[test]
    fn test_oklab_parsing() -> Result<()> {
        let test_cases: Vec<TestCase<String, Oklab>> = vec![
            TestCase {
                input: "oklab(0.5 0.2 0.1)".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklab(0.5, 0.2, 0.1)".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklab(0.5, 0.2 0.1)".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklab(0.5 0.2 0.1 0.8)".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 0.8,
                },
            },
            TestCase {
                input: "oklab(0.5 0.2 0.1 / 0.8)".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 0.8,
                },
            },
            TestCase {
                input: "oklab(0.5, 0.2, 0.1, 0.5)".to_owned(),

                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "oklab(0 0 0)".to_owned(),
                output: Oklab {
                    l: 0.0,
                    a: 0.0,
                    b: 0.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklab(1 0.5 -0.5)".to_owned(),
                output: Oklab {
                    l: 1.0,
                    a: 0.5,
                    b: -0.5,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "  oklab(0.5 0.2 0.1)  ".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "OKLAB(0.5 0.2 0.1)".to_owned(),
                output: Oklab {
                    l: 0.5,
                    a: 0.2,
                    b: 0.1,
                    alpha: 1.0,
                },
            },
        ];

        for tc in test_cases {
            assert_eq!(parse_oklab(&tc.input)?, tc.output);
        }

        Ok(())
    }

    #[test]
    fn test_oklch_parsing() -> Result<()> {
        let test_cases: Vec<TestCase<String, Oklch>> = vec![
            TestCase {
                input: "oklch(0.5 0.2 300)".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklch(0.5, 0.2, 300)".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklch(0.5, 0.2 300)".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklch(0.5 0.2 300 0.8)".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 0.8,
                },
            },
            TestCase {
                input: "oklch(0.5 0.2 300 / 0.8)".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 0.8,
                },
            },
            TestCase {
                input: "oklch(0.5, 0.2, 300, 0.5)".to_owned(),

                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 0.5,
                },
            },
            TestCase {
                input: "oklch(0 0 0)".to_owned(),
                output: Oklch {
                    l: 0.0,
                    c: 0.0,
                    h: 0.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "oklch(1 0.4 120)".to_owned(),
                output: Oklch {
                    l: 1.0,
                    c: 0.4,
                    h: 120.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "  oklch(0.5 0.2 300)  ".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 1.0,
                },
            },
            TestCase {
                input: "OKLCH(0.5 0.2 300)".to_owned(),
                output: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 1.0,
                },
            },
        ];

        for tc in test_cases {
            assert_eq!(parse_oklch(&tc.input)?, tc.output);
        }

        Ok(())
    }

    #[test]
    fn test_xyz_conversions() {
        let rgba = Rgba {
            r: 255,
            g: 0,
            b: 0,
            alpha: 1.0,
        };
        let xyz = rgba.to_xyz();
        let rgba_back = Rgba::from_xyz(&xyz, 1.0);

        assert!((rgba.r as f64 - rgba_back.r as f64).abs() < 1.0);
        assert!((rgba.g as f64 - rgba_back.g as f64).abs() < 1.0);
        assert!((rgba.b as f64 - rgba_back.b as f64).abs() < 1.0);

        let oklab = Oklab {
            l: 0.5,
            a: 0.1,
            b: 0.1,
            alpha: 1.0,
        };
        let xyz_lab = oklab.to_xyz();
        let oklab_back = Oklab::from_xyz(&xyz_lab, 1.0);

        assert!((oklab.l - oklab_back.l).abs() < 1e-4);
        assert!((oklab.a - oklab_back.a).abs() < 1e-4);
        assert!((oklab.b - oklab_back.b).abs() < 1e-4);

        let oklch = Oklch {
            l: 0.5,
            c: 0.1,
            h: 120.0,
            alpha: 1.0,
        };
        let xyz_lch = oklch.to_xyz();
        let oklch_back = Oklch::from_xyz(&xyz_lch, 1.0);

        assert!((oklch.l - oklch_back.l).abs() < 1e-4);
        assert!((oklch.c - oklch_back.c).abs() < 1e-4);
        assert!((oklch.h - oklch_back.h).abs() < 1e-1);
    }

    #[test]
    fn test_rgba_to_hex_string() {
        let test_cases = vec![
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0,
                },
                output: "#ffffff",
            },
            TestCase {
                input: Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
                output: "#000000",
            },
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
                output: "#ff0000",
            },
            TestCase {
                input: Rgba {
                    r: 0,
                    g: 255,
                    b: 0,
                    alpha: 1.0,
                },
                output: "#00ff00",
            },
            TestCase {
                input: Rgba {
                    r: 0,
                    g: 0,
                    b: 255,
                    alpha: 1.0,
                },
                output: "#0000ff",
            },
            TestCase {
                input: Rgba {
                    r: 1,
                    g: 2,
                    b: 3,
                    alpha: 1.0,
                },
                output: "#010203",
            },
            TestCase {
                input: Rgba {
                    r: 17,
                    g: 34,
                    b: 51,
                    alpha: 1.0,
                },
                output: "#112233",
            },
            TestCase {
                input: Rgba {
                    r: 10,
                    g: 20,
                    b: 30,
                    alpha: 1.0,
                },
                output: "#0a141e",
            },
            TestCase {
                input: Rgba {
                    r: 128,
                    g: 128,
                    b: 128,
                    alpha: 0.0,
                },
                output: "#80808000",
            },
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 0.5,
                },
                output: "#ffffff80",
            },
            TestCase {
                input: Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 0.5,
                },
                output: "#00000080",
            },
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0 / 3.0,
                },
                output: "#ffffff55",
            },
            TestCase {
                input: Rgba {
                    r: 12,
                    g: 34,
                    b: 56,
                    alpha: 0.25,
                },
                output: "#0c223840",
            },
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 0.9999,
                },
                output: "#ffffffff",
            },
            // Note: alpha is validated by parse_alpha and always in [0.0, 1.0]
            // (and hex parsing yields [0.0, 1.0] too), so no out-of-range cases here.
        ];

        for case in test_cases {
            assert_eq!(rgba_to_hex_string(&case.input), case.output);
        }
    }

    #[test]
    fn test_rgba_to_rgb_string() {
        let test_cases = vec![
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
                output: "rgb(255, 0, 0)",
            },
            TestCase {
                input: Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 1.0,
                },
                output: "rgb(0, 0, 0)",
            },
            TestCase {
                input: Rgba {
                    r: 1,
                    g: 2,
                    b: 3,
                    alpha: 0.0,
                },
                output: "rgb(1, 2, 3)",
            },
            TestCase {
                input: Rgba {
                    r: 10,
                    g: 20,
                    b: 30,
                    alpha: 0.5,
                },
                output: "rgb(10, 20, 30)",
            },
            TestCase {
                input: Rgba {
                    r: 128,
                    g: 64,
                    b: 32,
                    alpha: 0.25,
                },
                output: "rgb(128, 64, 32)",
            },
        ];

        for case in test_cases {
            assert_eq!(rgba_to_rgb_string(&case.input), case.output);
        }
    }

    #[test]
    fn test_rgba_to_rgba_string() {
        let test_cases = vec![
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    alpha: 1.0,
                },
                output: "rgba(255, 255, 255, 1)",
            },
            TestCase {
                input: Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    alpha: 0.0,
                },
                output: "rgba(0, 0, 0, 0)",
            },
            TestCase {
                input: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 0.5,
                },
                output: "rgba(255, 0, 0, 0.5)",
            },
            TestCase {
                input: Rgba {
                    r: 1,
                    g: 2,
                    b: 3,
                    alpha: 0.1,
                },
                output: "rgba(1, 2, 3, 0.1)",
            },
            TestCase {
                input: Rgba {
                    r: 10,
                    g: 20,
                    b: 30,
                    alpha: 1.0 / 3.0,
                },
                output: "rgba(10, 20, 30, 0.3333333333333333)",
            },
        ];

        for case in test_cases {
            assert_eq!(format!("{}", case.input), case.output);
        }
    }

    #[test]
    fn test_oklch_to_oklch_string() {
        let test_cases = vec![
            TestCase {
                input: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 1.0,
                },
                output: "oklch(0.5 0.2 300)",
            },
            TestCase {
                input: Oklch {
                    l: 0.0,
                    c: 0.0,
                    h: 0.0,
                    alpha: 1.0,
                },
                output: "oklch(0 0 0)",
            },
            TestCase {
                input: Oklch {
                    l: 1.0,
                    c: 0.4,
                    h: 360.0,
                    alpha: 1.0,
                },
                output: "oklch(1 0.4 360)",
            },
            TestCase {
                input: Oklch {
                    l: 0.5,
                    c: 0.2,
                    h: 300.0,
                    alpha: 0.5,
                },
                output: "oklch(0.5 0.2 300 / 0.5)",
            },
            TestCase {
                input: Oklch {
                    l: 0.7,
                    c: 0.1,
                    h: 120.0,
                    alpha: 0.0,
                },
                output: "oklch(0.7 0.1 120 / 0)",
            },
        ];

        for case in test_cases {
            assert_eq!(format!("{}", case.input), case.output);
        }
    }

    #[test]
    fn test_oklab_to_oklab_string() {
        let test_cases = vec![
            TestCase {
                input: Oklab {
                    l: 0.5,
                    a: 0.1,
                    b: 0.1,
                    alpha: 1.0,
                },
                output: "oklab(0.5 0.1 0.1)",
            },
            TestCase {
                input: Oklab {
                    l: 0.0,
                    a: 0.0,
                    b: 0.0,
                    alpha: 1.0,
                },
                output: "oklab(0 0 0)",
            },
            TestCase {
                input: Oklab {
                    l: 1.0,
                    a: -0.2,
                    b: 0.3,
                    alpha: 1.0,
                },
                output: "oklab(1 -0.2 0.3)",
            },
            TestCase {
                input: Oklab {
                    l: 0.5,
                    a: 0.1,
                    b: -0.1,
                    alpha: 0.5,
                },
                output: "oklab(0.5 0.1 -0.1 / 0.5)",
            },
            TestCase {
                input: Oklab {
                    l: 0.7,
                    a: 0.0,
                    b: 0.0,
                    alpha: 0.0,
                },
                output: "oklab(0.7 0 0 / 0)",
            },
        ];

        for case in test_cases {
            assert_eq!(format!("{}", case.input), case.output);
        }
    }

    #[test]
    fn test_convert() -> Result<()> {
        let test_cases = vec![
            ("#ff0000", Color::Hex, "#ff0000"),
            ("#f00", Color::Hex, "#ff0000"),
            ("#00ff0080", Color::Hex, "#00ff0080"),
            // target Rgb: hex input's r/g/b carry through, alpha is dropped.
            ("#ff0000", Color::Rgb, "rgb(255, 0, 0)"),
            // target Rgba: hex input's implicit alpha (1.0) is now shown explicitly.
            ("#ff0000", Color::Rgba, "rgba(255, 0, 0, 1)"),
            ("#00ff0080", Color::Rgb, "rgb(0, 255, 0)"),
            // 0x80 / 255 = 0.5019607843137255
            (
                "#00ff0080",
                Color::Rgba,
                "rgba(0, 255, 0, 0.5019607843137255)",
            ),
            ("rgb(255, 0, 0)", Color::Rgb, "rgb(255, 0, 0)"),
            ("rgb(10, 20, 30)", Color::Rgb, "rgb(10, 20, 30)"),
            // target Rgba: rgb input's implicit alpha (1.0) is now shown explicitly.
            ("rgb(10, 20, 30)", Color::Rgba, "rgba(10, 20, 30, 1)"),
            // target Rgb: rgba input's alpha is dropped.
            ("rgba(10, 20, 30, 0.5)", Color::Rgb, "rgb(10, 20, 30)"),
            (
                "rgba(10, 20, 30, 0.5)",
                Color::Rgba,
                "rgba(10, 20, 30, 0.5)",
            ),
            (
                "#ff0000",
                Color::Oklab,
                "oklab(0.627988706625623 0.22487488313589232 0.1258529885009403)",
            ),
            (
                "#ff0000",
                Color::Oklch,
                "oklch(0.627988706625623 0.2576968912889696 29.23389948383784)",
            ),
            (
                "rgb(255, 0, 0)",
                Color::Oklab,
                "oklab(0.627988706625623 0.22487488313589232 0.1258529885009403)",
            ),
            (
                "oklab(0.5 0.1 0.1)",
                Color::Oklab,
                "oklab(0.5000010922212974 0.10002486431431906 0.1000439314724102)",
            ),
            (
                "oklab(0.5 0.1 0.1)",
                Color::Oklch,
                "oklch(0.5000010922212974 0.14147000284708502 45.00546046012987)",
            ),
            // target Hex: must now produce a hex string, not rgba(...).
            // r=161=0xa1, g=66=0x42, b=3=0x03, alpha=1.0 -> 6-digit hex.
            ("oklab(0.5 0.1 0.1)", Color::Hex, "#a14203"),
            (
                "oklch(0.5 0.2 300)",
                Color::Oklch,
                "oklch(0.5000007475991664 0.19992041207423886 300.0117102735574)",
            ),
            (
                "oklch(0.5 0.2 300)",
                Color::Oklab,
                "oklab(0.5000007475991664 0.09999559000652258 -0.17311572182554913)",
            ),
            // target Hex: r=119=0x77, g=58=0x3a, b=193=0xc1, alpha=1.0 -> 6-digit hex.
            ("oklch(0.5 0.2 300)", Color::Hex, "#773ac1"),
        ];

        for (input, target, output) in test_cases {
            assert_eq!(convert(input, &target)?, output);
        }

        let invalid = ["not-a-color", "#12345", "rgb(1, 2)", "oklab(0.5 0.1)"];
        for input in invalid {
            assert!(convert(input, &Color::Hex).is_err());
        }

        Ok(())
    }

    #[test]
    fn test_batch_conversion_all_valid() -> Result<()> {
        let colors: Vec<String> = vec!["#ff0000", "rgb(0, 255, 0)", "rgba(0, 0, 255, 0.5)"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let results = convert_colors(&colors, &Color::Hex);

        assert_eq!(results.len(), colors.len());
        for (i, r) in results.iter().enumerate() {
            assert!(r.is_ok(), "expected success for {}: {:?}", colors[i], r);
        }

        // Spot-check that each result pairs the original input with a converted value.
        let (original, converted) = results[0].as_ref().unwrap();
        assert_eq!(original, "#ff0000");
        assert!(converted.starts_with('#'));

        let (_, converted_alpha) = results[2].as_ref().unwrap();
        // rgba with alpha < 1 should produce an 8-digit hex (with alpha channel)
        assert_eq!(converted_alpha.len(), 9); // "#" + 8 hex digits

        Ok(())
    }

    #[test]
    fn test_batch_conversion_mixed_valid_invalid() -> Result<()> {
        let colors: Vec<String> = vec![
            "#ff0000".to_string(),
            "not-a-color".to_string(),
            "rgb(0, 255, 0)".to_string(),
            "oklab(0.5 0.1)".to_string(), // invalid arity
        ];

        let results = convert_colors(&colors, &Color::Hex);

        assert_eq!(results.len(), colors.len());
        assert!(results[0].is_ok(), "expected #ff0000 to succeed");
        assert!(results[1].is_err(), "expected not-a-color to fail");
        assert!(results[2].is_ok(), "expected rgb(...) to succeed");
        assert!(results[3].is_err(), "expected malformed oklab to fail");

        // Order must be preserved — index in results matches index in input.
        let (original, _) = results[0].as_ref().unwrap();
        assert_eq!(original, "#ff0000");

        Ok(())
    }

    #[test]
    fn test_conversion_logs() -> Result<()> {
        let colors: Vec<String> = vec!["#fff", "rgba(255,255,255, 1)"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Oklab);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();

        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "expected no errors, got: {errors}");
        assert!(successes.contains("#fff ->"));
        assert!(successes.contains("rgba(255,255,255, 1) ->"));

        Ok(())
    }

    #[test]
    fn test_conversion_logs_with_invalid_color() -> Result<()> {
        let colors: Vec<String> = vec!["#fff", "not-a-color", "rgba(255,255,255, 1)"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Oklab);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();

        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(successes.contains("#fff ->"));
        assert!(successes.contains("rgba(255,255,255, 1) ->"));
        assert_eq!(errors.lines().count(), 1);
        assert!(errors.starts_with("error:"));

        Ok(())
    }

    #[test]
    fn test_conversion_logs_all_invalid() -> Result<()> {
        let colors: Vec<String> = vec!["garbage", "#zzz", "rgba(1,2,3,4,5)"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Oklch);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();

        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(
            successes.is_empty(),
            "expected no successes, got: {successes}"
        );
        assert_eq!(errors.lines().count(), 3);

        Ok(())
    }

    #[test]
    fn test_conversion_logs_empty_input() -> Result<()> {
        let colors: Vec<String> = Vec::new();
        let results = convert_colors(&colors, &Color::Rgb);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();

        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(successes.is_empty());
        assert!(errors.is_empty());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_hex_to_hex_roundtrip() -> Result<()> {
        let colors: Vec<String> = vec!["#ff0000", "#00ff00", "#0000ff"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Hex);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();

        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "expected no errors, got: {errors}");
        assert_eq!(successes.lines().count(), 3);
        assert!(successes.contains("#ff0000 -> #ff0000"));
        assert!(successes.contains("#00ff00 -> #00ff00"));
        assert!(successes.contains("#0000ff -> #0000ff"));

        Ok(())
    }

    #[test]
    fn test_conversion_logs_preserves_order() -> Result<()> {
        let colors: Vec<String> = vec!["#fff", "bad-input", "#000"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Oklab);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();

        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let success_lines: Vec<&str> = successes.lines().collect();

        assert_eq!(success_lines.len(), 2);
        assert!(success_lines[0].starts_with("#fff ->"));
        assert!(success_lines[1].starts_with("#000 ->"));

        Ok(())
    }

    #[test]
    fn test_conversion_logs_hex_edge_cases() -> Result<()> {
        let colors: Vec<String> = vec![
            "#fff",
            "#000",
            "#ffff",
            "#0000",
            "#ffffff",
            "#000000",
            "#ffffffff",
            "#00000000",
            "#ABC",
            "#FfFfFf",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Hex);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "unexpected errors: {errors}");
        assert_eq!(successes.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_hex_invalid_lengths() -> Result<()> {
        let colors: Vec<String> = vec!["#f", "#ff", "#12345", "#1234567", "#123456789", "#gggggg"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Hex);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(successes.is_empty(), "expected no successes: {successes}");
        assert_eq!(errors.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_rgb_edge_cases() -> Result<()> {
        let colors: Vec<String> = vec![
            "rgb(0, 0, 0)",
            "rgb(255, 255, 255)",
            "rgb(0.4, 0.5, 0.6)",
            "rgb  (1 0 255)  ",
            "rgb(255,0,0)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Rgb);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "unexpected errors: {errors}");
        assert_eq!(successes.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_rgb_invalid_arity() -> Result<()> {
        let colors: Vec<String> = vec!["rgb()", "rgb(255)", "rgb(255, 0)", "rgb(255, 0, 0, 0)"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Rgb);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(successes.is_empty(), "expected no successes: {successes}");
        assert_eq!(errors.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_rgba_alpha_boundaries() -> Result<()> {
        let colors: Vec<String> = vec![
            "rgba(255, 255, 255, 0)",
            "rgba(255, 255, 255, 1)",
            "rgba(255, 255, 255, 1.0)",
            "rgba(255, 255, 255, 50%)",
            "rgba(255, 255, 255, -0.1)", // invalid: out of range
            "rgba(255, 255, 255, 1.1)",  // invalid: out of range
            "rgba(255, 255, 255, 200%)", // invalid: out of range
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Rgba);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert_eq!(successes.lines().count(), 4);
        assert_eq!(errors.lines().count(), 3);

        Ok(())
    }

    #[test]
    fn test_conversion_logs_oklab_edge_cases() -> Result<()> {
        let colors: Vec<String> = vec![
            "oklab(0 0 0)",
            "oklab(1 0 0)",
            "oklab(0.5 -0.2 -0.1)",
            "oklab(-0.1 0.2 0.1)",
            "oklab(50% 0.2 0.1% / 0%)",
            "oklab(0.5 0.2 0.1 / 100%)",
            "OKLAB(0.5 0.2 0.1)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Oklab);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "unexpected errors: {errors}");
        assert_eq!(successes.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_oklab_invalid_arity() -> Result<()> {
        let colors: Vec<String> = vec![
            "oklab()",
            "oklab(0.5)",
            "oklab(0.5 0.2)",
            "oklab(0.5 0.2 0.1 0.5 0.5)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Oklab);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(successes.is_empty(), "expected no successes: {successes}");
        assert_eq!(errors.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_oklch_edge_cases() -> Result<()> {
        let colors: Vec<String> = vec![
            "oklch(0.5 0.2 0)",
            "oklch(0.5 0.2 360)",
            "oklch(0.5 0 120)",
            "oklch(0.5 0.2 30deg)",
            "oklch(0.5 0.2 300 / 0.5)",
            "oklch(0.5, 0.2, 300, 1)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Oklch);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "unexpected errors: {errors}");
        assert_eq!(successes.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_oklch_invalid_arity() -> Result<()> {
        let colors: Vec<String> = vec![
            "oklch()",
            "oklch(0.5)",
            "oklch(0.5 0.2)",
            "oklch(0.5 0.2 300 0.5 0.5)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let results = convert_colors(&colors, &Color::Oklch);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(successes.is_empty(), "expected no successes: {successes}");
        assert_eq!(errors.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_oklch_negative_hue() -> Result<()> {
        let colors: Vec<String> = vec!["oklch(0.5 0.2 -30deg)", "oklch(0.5 0.2 -300)"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let results = convert_colors(&colors, &Color::Oklch);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert!(errors.is_empty(), "unexpected errors: {errors}");
        assert_eq!(successes.lines().count(), colors.len());

        Ok(())
    }

    #[test]
    fn test_conversion_logs_cross_type_batch() -> Result<()> {
        let colors: Vec<String> = vec![
            "#ff0000",
            "rgb(0, 255, 0)",
            "rgba(0, 0, 255, 0.5)",
            "oklab(0.5 0.1 0.1)",
            "oklch(0.5 0.2 300)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        for target in [
            Color::Hex,
            Color::Rgb,
            Color::Rgba,
            Color::Oklab,
            Color::Oklch,
        ] {
            let results = convert_colors(&colors, &target);

            let mut w_buf = Vec::new();
            let mut l_buf = Vec::new();
            log_conversion_results(&results, &mut w_buf, &mut l_buf);

            let successes = String::from_utf8(w_buf)?;
            let errors = String::from_utf8(l_buf)?;

            assert!(
                errors.is_empty(),
                "target {target:?}: unexpected errors: {errors}"
            );
            assert_eq!(
                successes.lines().count(),
                colors.len(),
                "target {target:?}: expected all {} inputs to succeed",
                colors.len()
            );
        }

        Ok(())
    }

    #[test]
    fn test_conversion_logs_mixed_valid_and_invalid_all_types() -> Result<()> {
        let colors: Vec<String> = vec![
            "#fff".to_string(),
            "#12345".to_string(),
            "rgb(1, 2, 3)".to_string(),
            "rgb(1, 2)".to_string(),
            "rgba(1, 2, 3, 0.5)".to_string(),
            "rgba(1, 2, 3, 1.5)".to_string(),
            "oklab(0.5 0.1 0.1)".to_string(),
            "oklab(0.5 0.1)".to_string(),
            "oklch(0.5 0.2 300)".to_string(),
            "oklch(0.5 0.2)".to_string(),
            "not-a-color".to_string(),
        ];
        let results = convert_colors(&colors, &Color::Oklab);

        let mut w_buf = Vec::new();
        let mut l_buf = Vec::new();
        log_conversion_results(&results, &mut w_buf, &mut l_buf);

        let successes = String::from_utf8(w_buf)?;
        let errors = String::from_utf8(l_buf)?;

        assert_eq!(successes.lines().count(), 5);
        assert_eq!(errors.lines().count(), 6);

        let success_lines: Vec<&str> = successes.lines().collect();
        assert!(success_lines[0].starts_with("#fff ->"));
        assert!(success_lines[1].starts_with("rgb(1, 2, 3) ->"));
        assert!(success_lines[2].starts_with("rgba(1, 2, 3, 0.5) ->"));
        assert!(success_lines[3].starts_with("oklab(0.5 0.1 0.1) ->"));
        assert!(success_lines[4].starts_with("oklch(0.5 0.2 300) ->"));

        Ok(())
    }

    #[test]
    fn test_oklab_oklch_regex_no_cross_match() -> Result<()> {
        // Ensures the -? additions to OKLAB_REGEX/OKLCH_REGEX didn't cause
        // one prefix to be misidentified as the other.
        assert_eq!(Color::Oklch, identify_color("oklch(0.5 0.2 -30deg)")?);
        assert_eq!(Color::Oklch, identify_color("oklch(0.5 0.2 -300)")?);
        assert_eq!(Color::Oklab, identify_color("oklab(0.5 -0.2 -0.1)")?);
        assert_eq!(Color::Oklab, identify_color("oklab(-0.1 0.2 0.1)")?);

        Ok(())
    }
}
