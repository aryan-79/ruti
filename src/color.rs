use anyhow::{Ok, Result, bail};
use clap::ValueEnum;
use regex::Regex;

use std::sync::LazyLock;
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

static HEX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3}|[A-Fa-f0-9]{4}|[A-Fa-f0-9]{8})$").unwrap()
});

static RGB_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^rgb\s*\(\s*([.\d]+)[, ]+([.\d]+)[, ]+([.\d]+)\s*\)\s*$").unwrap()
});

static RGBA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^rgba\s*\(\s*([.\d]+)[, ]+([.\d]+)[, ]+([.\d]+)[, ]+([.\d]+)\s*\)\s*$")
        .unwrap()
});

static OKLAB_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^oklab\s*\(\s*([.\d]+%?)[, ]+([.\d]+%?)[, ]+([.\d]+%?)\s*(?:[/,]\s*([.\d]+%?)\s*)?\)\s*$").unwrap()
});

static OKLCH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^oklch\s*\(\s*([.\d]+%?)[, ]+([.\d]+%?)[, ]+([.\d]+(?:deg)?%?)\s*(?:[/,]\s*([.\d]+%?)\s*)?\)\s*$").unwrap()
});

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
    let alpha = parts
        .get(3)
        .map_or(Ok(1.0), |a| parse_unit_aware(a, 1.0, |v| v))?;

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
    let alpha = parts
        .get(3)
        .map_or(Ok(1.0), |a| parse_unit_aware(a, 1.0, |v| v))?;

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
    let alpha = parts
        .get(3)
        .map_or(Ok(1.0), |a| parse_unit_aware(a, 1.0, |v| v))?;

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

        assert_eq!(Color::Oklab, identify_color(&space_sep)?);
        assert_eq!(Color::Oklab, identify_color(&comma_sep)?);
        assert_eq!(Color::Oklab, identify_color(&with_alpha)?);
        assert_eq!(Color::Oklab, identify_color(&comma_alpha)?);
        assert_eq!(Color::Oklab, identify_color(&with_percent)?);
        assert_eq!(Color::Oklab, identify_color(&mixed_sep)?);

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
            "oklch(1 2 3 4)",
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
                input: "rgba(255, 0, 0, 128)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 128.0,
                },
            },
            TestCase {
                input: "rgba(1 3 255 128)".to_owned(),
                output: Rgba {
                    r: 1,
                    g: 3,
                    b: 255,
                    alpha: 128.0,
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
                input: "rgba (255, 0, 0, 128)".to_owned(),
                output: Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    alpha: 128.0,
                },
            },
            TestCase {
                input: "rgba  (1 3, 255, 128)  ".to_owned(),
                output: Rgba {
                    r: 1,
                    g: 3,
                    b: 255,
                    alpha: 128.0,
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
}
