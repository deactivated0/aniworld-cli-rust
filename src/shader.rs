//! Video GLSL Shaders for mpv playback enhancement (Anime4K, Sharpening, Line Art, Denoise, Vivid).
use std::{fs, path::PathBuf};

use anyhow::Result;

use crate::state;

pub const ANIME4K_UPSCALE_GLSL: &str = r#"//!HOOK OUTPUT
//!BIND HOOKED
//!DESC Anime4K Fast Reconstruction & Upscale

vec4 hook() {
    vec2 pos = HOOKED_pos;
    vec2 pt = HOOKED_pt;
    vec4 c = HOOKED_tex(pos);
    
    vec4 n = HOOKED_tex(pos + vec2(0.0, -pt.y));
    vec4 s = HOOKED_tex(pos + vec2(0.0, pt.y));
    vec4 e = HOOKED_tex(pos + vec2(pt.x, 0.0));
    vec4 w = HOOKED_tex(pos + vec2(-pt.x, 0.0));
    
    vec4 g = abs(n - s) + abs(e - w);
    float edge = dot(g.rgb, vec3(0.333));
    
    vec4 min_val = min(c, min(min(n, s), min(e, w)));
    vec4 max_val = max(c, max(max(n, s), max(e, w)));
    vec4 sharp = c + (c - 0.25 * (n + s + e + w)) * (edge * 2.0);
    
    return clamp(sharp, min_val, max_val);
}
"#;

pub const ADAPTIVE_SHARPEN_GLSL: &str = r#"//!HOOK OUTPUT
//!BIND HOOKED
//!DESC Anime Adaptive Sharpening (Edge-Preserving Detail Enhancement)

vec4 hook() {
    vec2 pos = HOOKED_pos;
    vec2 pt = HOOKED_pt;
    vec4 c = HOOKED_tex(pos);
    vec4 n = HOOKED_tex(pos + vec2(0.0, -pt.y));
    vec4 s = HOOKED_tex(pos + vec2(0.0, pt.y));
    vec4 e = HOOKED_tex(pos + vec2(pt.x, 0.0));
    vec4 w = HOOKED_tex(pos + vec2(-pt.x, 0.0));
    vec4 nw = HOOKED_tex(pos + vec2(-pt.x, -pt.y));
    vec4 ne = HOOKED_tex(pos + vec2(pt.x, -pt.y));
    vec4 sw = HOOKED_tex(pos + vec2(-pt.x, pt.y));
    vec4 se = HOOKED_tex(pos + vec2(pt.x, pt.y));
    
    vec4 min_val = min(c, min(min(n, s), min(e, w)));
    vec4 max_val = max(c, max(max(n, s), max(e, w)));
    
    vec4 laplace = 4.0 * (n + s + e + w) + (nw + ne + sw + se) - 20.0 * c;
    vec4 sharpened = c + laplace * -0.08;
    return clamp(sharpened, min_val, max_val);
}
"#;

pub const LINE_DARKEN_GLSL: &str = r#"//!HOOK OUTPUT
//!BIND HOOKED
//!DESC Anime Line Art Darken & Edge Clarity

vec4 hook() {
    vec2 pos = HOOKED_pos;
    vec2 pt = HOOKED_pt;
    vec4 c = HOOKED_tex(pos);
    
    vec4 n = HOOKED_tex(pos + vec2(0.0, -pt.y));
    vec4 s = HOOKED_tex(pos + vec2(0.0, pt.y));
    vec4 e = HOOKED_tex(pos + vec2(pt.x, 0.0));
    vec4 w = HOOKED_tex(pos + vec2(-pt.x, 0.0));
    
    float lum_n = dot(n.rgb, vec3(0.299, 0.587, 0.114));
    float lum_s = dot(s.rgb, vec3(0.299, 0.587, 0.114));
    float lum_e = dot(e.rgb, vec3(0.299, 0.587, 0.114));
    float lum_w = dot(w.rgb, vec3(0.299, 0.587, 0.114));
    
    float gx = lum_e - lum_w;
    float gy = lum_s - lum_n;
    float edge = sqrt(gx * gx + gy * gy);
    
    float line_weight = clamp(edge * 1.5, 0.0, 0.35);
    return vec4(c.rgb * (1.0 - line_weight), c.a);
}
"#;

pub const DENOISE_GLSL: &str = r#"//!HOOK OUTPUT
//!BIND HOOKED
//!DESC Anime Denoise & Artifact Reduction

vec4 hook() {
    vec2 pos = HOOKED_pos;
    vec2 pt = HOOKED_pt;
    vec4 c = HOOKED_tex(pos);
    
    vec4 sum = c * 4.0;
    float total_w = 4.0;
    
    vec2 offsets[4] = vec2[](
        vec2(0.0, -pt.y),
        vec2(0.0, pt.y),
        vec2(pt.x, 0.0),
        vec2(-pt.x, 0.0)
    );
    
    for (int i = 0; i < 4; i++) {
        vec4 sample_c = HOOKED_tex(pos + offsets[i]);
        float diff = distance(c.rgb, sample_c.rgb);
        float w = exp(-diff * 8.0);
        sum += sample_c * w;
        total_w += w;
    }
    
    return sum / total_w;
}
"#;

pub const VIVID_GLSL: &str = r#"//!HOOK OUTPUT
//!BIND HOOKED
//!DESC Anime Vibrant Color & Contrast Enhancement

vec4 hook() {
    vec4 c = HOOKED_tex(HOOKED_pos);
    vec3 rgb = c.rgb;
    rgb = rgb * rgb * (3.0 - 2.0 * rgb);
    
    float gray = dot(rgb, vec3(0.299, 0.587, 0.114));
    rgb = mix(vec3(gray), rgb, 1.18);
    
    return vec4(clamp(rgb, 0.0, 1.0), c.a);
}
"#;

#[derive(Clone, Copy)]
pub struct ShaderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub file_name: &'static str,
    pub source: &'static str,
}

pub const PRESETS: &[ShaderPreset] = &[
    ShaderPreset {
        id: "anime4k",
        name: "Anime4K (Fast Edge Reconstruction & Upscale)",
        file_name: "anime4k_upscale.glsl",
        source: ANIME4K_UPSCALE_GLSL,
    },
    ShaderPreset {
        id: "adaptive-sharpen",
        name: "Adaptive Sharpen (CAS - Crisp Details)",
        file_name: "anime_adaptive_sharpen.glsl",
        source: ADAPTIVE_SHARPEN_GLSL,
    },
    ShaderPreset {
        id: "line-darken",
        name: "Anime Line Art Darken (Deep Contours)",
        file_name: "anime_line_darken.glsl",
        source: LINE_DARKEN_GLSL,
    },
    ShaderPreset {
        id: "denoise",
        name: "Anime Denoise & Clean (Artifact Reduction)",
        file_name: "anime_denoise.glsl",
        source: DENOISE_GLSL,
    },
    ShaderPreset {
        id: "vivid",
        name: "Anime Vivid Colors & Dynamic Range",
        file_name: "anime_vivid.glsl",
        source: VIVID_GLSL,
    },
];

pub fn ensure_shaders_installed() -> Result<PathBuf> {
    let dir = state::config_dir().join("shaders");
    fs::create_dir_all(&dir)?;
    for preset in PRESETS {
        let path = dir.join(preset.file_name);
        if !path.exists() {
            let _ = fs::write(&path, preset.source);
        }
    }
    Ok(dir)
}

pub fn resolve_shader_path(target: &str) -> Option<PathBuf> {
    let _ = ensure_shaders_installed();
    let dir = state::config_dir().join("shaders");

    // Match by preset ID, file name, or descriptive name
    for preset in PRESETS {
        if target.eq_ignore_ascii_case(preset.id)
            || target.eq_ignore_ascii_case(preset.file_name)
            || target.contains(preset.file_name)
            || target.contains(preset.name)
        {
            let path = dir.join(preset.file_name);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // Direct existing file path
    let p = PathBuf::from(target);
    if p.is_file() {
        return Some(p);
    }

    // In aniworld shaders dir
    let in_dir = dir.join(target);
    if in_dir.is_file() {
        return Some(in_dir);
    }

    // In mpv config shaders dir (~/.config/mpv/shaders/...)
    #[cfg(not(target_os = "windows"))]
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = target
            .trim_start_matches("~~/shaders/")
            .trim_start_matches("~~/");
        let mpv_shader = PathBuf::from(home)
            .join(".config")
            .join("mpv")
            .join("shaders")
            .join(trimmed);
        if mpv_shader.is_file() {
            return Some(mpv_shader);
        }
    }

    // Fallback to the default Anime4K shader
    let fallback = dir.join("anime4k_upscale.glsl");
    if fallback.is_file() {
        Some(fallback)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_exist_and_have_hooks() {
        assert_eq!(PRESETS.len(), 5);
        for p in PRESETS {
            assert!(p.source.contains("//!HOOK"));
            assert!(p.source.contains("vec4 hook()"));
            assert!(!p.file_name.is_empty());
        }
    }

    #[test]
    fn test_resolve_shader_preset() {
        let p = resolve_shader_path("anime4k");
        assert!(p.is_some());
        let p = resolve_shader_path("adaptive-sharpen");
        assert!(p.is_some());
    }
}
