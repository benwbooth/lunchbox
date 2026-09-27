//! Render external artwork inside Koko, where the CRT light can illuminate it.
//! An ordinary RetroArch overlay is composited afterwards and hides that light.

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, ensure};

/// Adapt Koko's final composition pass. All CRT/curvature passes and the
/// upstream files remain untouched. Reject unfamiliar layouts rather than
/// silently loading a shader with broken screen geometry.
pub(crate) fn final_pass(source: &str, shader_directory: &Path, ultrawide: bool) -> Result<String> {
    let background = "get_scaled_coords_aspect_fgbg(vBg_img_coords, global.FinalViewportSize, image_aspect, rotated_bg, vIsRotated, vIn_aspect)";
    let coordinates = "    vOutputCoord = antiburn_TexCoord ;";
    let light = "vec3 light = pixel_ambi.rgb * (ambi_mask) * (1- fg_image_alpha_adapted);";
    ensure!(
        source.matches(background).count() == 1,
        "Unsupported Koko artwork composition layout"
    );
    ensure!(
        source.matches(coordinates).count() == 1,
        "Unsupported Koko viewport layout"
    );
    ensure!(
        source.matches(light).count() == 1,
        "Unsupported Koko artwork lighting layout"
    );
    let mut adapted = source.replace(
        background,
        // Koko's default background follows the content aspect on narrow
        // outputs. External artwork must instead always fit its own aspect.
        "get_scaled_coords_aspect(vBg_img_coords, global.FinalViewportSize, image_aspect, false)",
    );
    let fragment = "layout(location = 0)     out vec4 FragColor;";
    let content = "vec2 co_content = vTexCoord;";
    let reuse = "if (vDeltaRenderOk > 0.0 && content_mask > 1.0 - eps ){";
    ensure!(
        source.matches(content).count() == 1
            && source.matches(fragment).count() == 1
            && source.matches(reuse).count() == 1,
        "Unsupported Koko inner lip interface"
    );
    // The padding itself never changes, but the picture reflected in it does.
    // Reusing those black source pixels would freeze the inner-lip reflection.
    // Repaint the whole possible lip band, including when padding disappears
    // during a fade to black, so a previous reflection cannot remain there.
    adapted = adapted.replace(reuse, "if (vDeltaRenderOk > 0.0 && content_mask > 1.0 - eps && is_first_inside_rect(co_content, vec4(0.08, 0.08, 0.92, 0.92))) {");
    adapted = adapted.replace(
        fragment,
        &format!(
            "layout(set = 0, binding = 14) uniform sampler2D lunchpail_picture_bounds;\n{fragment}"
        ),
    );
    adapted = adapted.replace(
        content,
        &format!("vec4 lunchpailPictureBounds = texelFetch(lunchpail_picture_bounds, ivec2(2), 0);\n{content}"),
    );
    adapted = adapted.replace(light, include_str!("retrotube_inner_reflection.slang"));
    for include in ["config.inc", "includes/functions.include.slang"] {
        let directive = format!("#include \"{include}\"");
        ensure!(
            adapted.contains(&directive),
            "Missing Koko include {include}"
        );
        let path = shader_directory
            .join(include)
            .to_string_lossy()
            .replace('\\', "/");
        adapted = adapted.replace(&directive, &format!("#include \"{path}\""));
    }
    if ultrawide {
        // This is the same centered opening as the old custom viewport, now
        // calculated by the GPU so fullscreen/window resizing stays correct.
        let fit = format!(
            "{}\n{coordinates}",
            opening_fit("vTexCoord", "vIn_aspect", "bIsRotated")
        );
        adapted = adapted.replace(coordinates, &fit);
    }
    Ok(adapted)
}

fn opening_fit(coordinates: &str, aspect: &str, rotated: &str) -> String {
    let (art_width, art_height) = crate::bezel_orionsangel::ULTRAWIDE_DIMENSIONS;
    let (_, _, hole_width, hole_height) = crate::bezel_orionsangel::ULTRAWIDE_SCREEN_OPENING;
    format!(
        "    vec2 lunchpailOutput = global.FinalViewportSize.xy;\n\
         float lunchpailArtScale = min(lunchpailOutput.x / {art_width}.0, lunchpailOutput.y / {art_height}.0);\n\
         vec2 lunchpailOpening = vec2({hole_width}.0, {hole_height}.0) * lunchpailArtScale;\n\
         float lunchpailAspect = {rotated} ? 1.0 / {aspect} : {aspect};\n\
         vec2 lunchpailOriginal = vec2(lunchpailAspect, 1.0) * min(lunchpailOutput.x / lunchpailAspect, lunchpailOutput.y);\n\
         {coordinates} = ({coordinates} - 0.5) * lunchpailOriginal / lunchpailOpening + 0.5;\n"
    )
}

pub(crate) fn install(base: &Path, preset: &Path, image: &Path, ultrawide: bool) -> Result<()> {
    let koko = base
        .parent()
        .and_then(Path::parent)
        .context("Missing Koko directory")?;
    let engine = fs::read_to_string(koko.join("koko-aio-ng.slangp"))?;
    let engine_settings = assignments(&engine);
    ensure!(
        engine_settings.get("shaders").map(String::as_str) == Some("17")
            && engine_settings.get("shader16").map(String::as_str)
                == Some("shaders-ng/final_pass.slang"),
        "Unsupported Koko final pass index"
    );
    let directory = koko.join("shaders-ng");
    let source = fs::read_to_string(directory.join("final_pass.slang"))?;
    let shader = final_pass(&source, &directory, ultrawide)?;
    let shader_path = preset.with_extension("slang");
    fs::write(&shader_path, shader)?;
    // Reference presets may override parameters/textures, but not shader passes.
    // Materialize the chain before replacing the final compositor. Keep every
    // upstream pass and resolve its paths relative to the file that defined it.
    let mut settings = read_preset(preset, 0)?;
    // Insert a tiny analysis pass before the final compositor. RetroArch
    // forbids vertex texture reads; scanning in the fullscreen fragment pass
    // would repeat the work millions of times. Aliases keep feedback intact.
    for prefix in [
        "shader",
        "alias",
        "filter_linear",
        "wrap_mode",
        "mipmap_input",
        "float_framebuffer",
        "srgb_framebuffer",
        "scale_type",
        "scale_type_x",
        "scale_type_y",
        "scale",
        "scale_x",
        "scale_y",
        "frame_count_mod",
    ] {
        if let Some(value) = settings.remove(&format!("{prefix}16")) {
            settings.insert(format!("{prefix}17"), value);
        }
    }
    let bounds_path = preset.with_extension("inner-lip.slang");
    fs::write(&bounds_path, include_str!("retrotube_inner_lip.slang"))?;
    settings.insert("shaders".into(), "18".into());
    settings.insert(
        "shader16".into(),
        bounds_path.to_string_lossy().replace('\\', "/"),
    );
    settings.insert("alias16".into(), "lunchpail_picture_bounds".into());
    settings.insert("scale_type16".into(), "absolute".into());
    settings.insert("scale16".into(), "4".into());
    settings.insert("float_framebuffer16".into(), "true".into());
    settings.insert("filter_linear16".into(), "false".into());
    if ultrawide {
        let source = fs::read_to_string(directory.join("ambi_temporal_pass.slang"))?;
        let anchor = "    if (bNeed_NO_integer_scale) {";
        ensure!(
            source.matches(anchor).count() == 1,
            "Unsupported Koko lighting geometry"
        );
        ensure!(
            settings
                .get("shader14")
                .is_some_and(|path| path.ends_with("/ambi_temporal_pass.slang")),
            "Unsupported Koko lighting pass index"
        );
        let mut source = source.replace(
            anchor,
            &format!(
                "{}\n{anchor}",
                opening_fit("pre_pass_coords", "in_aspect", "isrotated")
            ),
        );
        for include in ["config.inc", "includes/functions.include.slang"] {
            let absolute = directory.join(include).to_string_lossy().replace('\\', "/");
            source = source.replace(
                &format!("#include \"{include}\""),
                &format!("#include \"{absolute}\""),
            );
        }
        let path = preset.with_extension("ambient.slang");
        fs::write(&path, source)?;
        settings.insert("shader14".into(), path.to_string_lossy().replace('\\', "/"));
    }
    let image = image.to_string_lossy().replace('\\', "/");
    let shader = shader_path.to_string_lossy().replace('\\', "/");
    let overrides = format!(
        "\n# Artwork participates in CRT lighting instead of hiding it.\n\
         shader17 = \"{shader}\"\n\
         bg_over = \"{image}\"\n\
         DO_BEZEL = \"0.0\"\n\
         DO_BG_IMAGE = \"1.0\"\n\
         BG_IMAGE_OVER = \"1.0\"\n\
         BG_IMAGE_ROTATION = \"0.0\"\n\
         BG_IMAGE_ZOOM = \"1.0\"\n\
         BG_IMAGE_WRAP_MODE = \"1.0\"\n\
         BG_IMAGE_NIGHTIFY = \"0.0\"\n\
         DO_AMBILIGHT = \"1.0\"\n\
         AMBI_BG_IMAGE_BLEND_MODE = \"1.0\"\n\
         AMBI_BG_IMAGE_FORCE = \"0.65\"\n\
         AMBI_ADD_ON_BLACK = \"0.12\"\n\
         ASPECT_X = \"-8.0\"\n"
    );
    settings.extend(assignments(&overrides));
    let settings: String = settings
        .into_iter()
        .map(|(key, value)| format!("{key} = \"{value}\"\n"))
        .collect();
    fs::write(preset, settings)?;
    Ok(())
}

fn assignments(contents: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for line in contents.lines().map(str::trim) {
        if line.starts_with(['#', '/']) {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values
                .entry(key.trim().to_owned())
                .or_insert_with(|| value.trim().trim_matches('"').to_owned());
        }
    }
    values
}

fn read_preset(path: &Path, depth: usize) -> Result<BTreeMap<String, String>> {
    ensure!(
        depth < 16,
        "Cyclic or excessively nested shader preset references"
    );
    let contents =
        fs::read_to_string(path).with_context(|| format!("Reading {}", path.display()))?;
    let parent = path
        .parent()
        .context("Shader preset has no parent directory")?;
    let mut values = BTreeMap::new();
    for line in contents.lines().map(str::trim) {
        if let Some(reference) = line.strip_prefix("#reference ") {
            values.extend(read_preset(
                &parent.join(reference.trim().trim_matches('"')),
                depth + 1,
            )?);
        }
    }
    let own = assignments(&contents);
    let textures = own
        .get("textures")
        .or_else(|| values.get("textures"))
        .cloned()
        .unwrap_or_default();
    for (key, mut value) in own {
        let shader = key
            .strip_prefix("shader")
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()));
        if shader || textures.split(';').any(|texture| texture == key) {
            value = parent.join(value).to_string_lossy().replace('\\', "/");
        }
        values.insert(key, value);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = "#include \"config.inc\"\n#include \"includes/functions.include.slang\"\nlayout(location = 0)     out vec4 FragColor;\nvec2 co_content = vTexCoord;\nif (vDeltaRenderOk > 0.0 && content_mask > 1.0 - eps ){\n    vOutputCoord = antiburn_TexCoord ;\nimage_coords = get_scaled_coords_aspect_fgbg(vBg_img_coords, global.FinalViewportSize, image_aspect, rotated_bg, vIsRotated, vIn_aspect);\nvec3 light = pixel_ambi.rgb * (ambi_mask) * (1- fg_image_alpha_adapted);";

    #[test]
    fn artwork_fit_does_not_stretch_or_change_regular_game_geometry() {
        let adapted = final_pass(SOURCE, Path::new("/shaders/koko"), false).unwrap();
        assert!(adapted.contains("get_scaled_coords_aspect(vBg_img_coords"));
        assert!(!adapted.contains("lunchpailOpening"));
        assert!(adapted.contains("#include \"/shaders/koko/config.inc\""));
        assert!(adapted.contains("vec3 light = vec3(0.0)"));
        assert!(adapted.contains("* (lunchpailPadding + lunchpailBevel) * lunchpailArtBounds"));
        assert!(adapted.contains("texture(in_glow_pass, lunchpailSample)"));
        assert!(adapted.contains("lunchpailReflection * lunchpailLip"));
        assert!(adapted.contains("texelFetch(lunchpail_picture_bounds, ivec2(2), 0)"));
        assert!(adapted.contains("is_first_inside_rect(co_content, vec4(0.08, 0.08, 0.92, 0.92))"));
        assert!(!adapted.contains("lunchpailArtMask"));
    }

    #[test]
    fn inner_lip_uses_koko_material_controls_without_changing_screen_geometry() {
        let material = include_str!("retrotube_inner_reflection.slang");
        for control in [
            "BEZEL_REFL_STRENGTH",
            "BEZEL_RFL_BLR_SHD",
            "BEZEL_RFL_ZOOM",
            "BEZEL_ROUGHNESS",
            "BEZEL_CORNER_DARK",
        ] {
            assert!(
                material.contains(control),
                "Missing material control {control}"
            );
        }
        assert!(material.contains("2.0 * lunchpailEdge - co_content"));
        assert!(material.contains("corners_shade(lunchpailSurface, 1.125)"));
        assert!(material.contains("direction < 8"));
        assert!(material.contains("random_fast(vTexCoord)"));
        assert!(
            !material.contains("FrameCount"),
            "Surface grain must not shimmer"
        );
        assert!(!material.contains("co_content ="));
        assert!(!material.contains("* 0.55"));
    }

    #[test]
    fn opaque_inner_bevel_does_not_require_native_black_padding() {
        let material = include_str!("retrotube_inner_reflection.slang");
        assert!(material.contains("* (1.0 - pixel_fg_image.a)"));
        assert!(material.contains("* pixel_fg_image.a"));
        assert!(material.contains("smoothstep(0.0, 0.055, lunchpailDistance)"));
        assert!(material.contains("smoothstep(0.10, 0.28, lunchpailArtBrightness)"));
        assert!(material.contains("step(0.00001, lunchpailDistance)"));
        // The opaque material adds to (not replaces) the existing transparent
        // lip. The same shader must handle a full-frame console and an arcade
        // core with native black margins, without per-game special cases.
        assert!(material.contains("lunchpailPadding + lunchpailBevel"));
    }

    #[test]
    fn ultrawide_opening_uses_render_dimensions_not_monitor_pixels() {
        let adapted = final_pass(SOURCE, Path::new("/shaders/koko"), true).unwrap();
        assert!(adapted.contains("global.FinalViewportSize.xy"));
        assert!(adapted.contains("vec2(1186.0, 888.0)"));
        assert!(adapted.contains("vTexCoord - 0.5"));
    }

    #[test]
    fn unfamiliar_shader_falls_back_instead_of_guessing_geometry() {
        assert!(final_pass("new shader layout", Path::new("/shaders"), true).is_err());
    }

    #[test]
    fn padding_analysis_is_bounded_and_never_zooms_the_game() {
        let shader = include_str!("retrotube_inner_lip.slang");
        assert!(shader.starts_with("#version 450"));
        assert!(shader.contains("cross < height"));
        assert!(shader.contains("texelFetch(Original, p, 0)"));
        assert!(shader.contains("max(size.x, size.y) > 1024"));
        assert!(!shader.contains("zoom"));
    }

    #[test]
    fn installation_replaces_disabled_lighting_without_duplicate_parameters() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("Presets-ng")).unwrap();
        fs::create_dir_all(root.join("shaders-ng")).unwrap();
        fs::write(root.join("koko-aio-ng.slangp"), "shaders = 17\nshader8 = shaders-ng/reflection_blur_pre.slang\nshader9 = shaders-ng/reflection_blur.slang\nshader16 = shaders-ng/final_pass.slang\ntextures = \"bg_over\"\nbg_over = \"textures/default.png\"\n").unwrap();
        for name in ["reflection_blur_pre.slang", "reflection_blur.slang"] {
            fs::write(
                root.join("shaders-ng").join(name),
                "if (DO_BEZEL == 0.0) return;",
            )
            .unwrap();
        }
        fs::write(
            root.join("Base.slangp"),
            "#reference \"koko-aio-ng.slangp\"\nDO_PIXELGRID = \"1.0\"\n",
        )
        .unwrap();
        fs::write(root.join("shaders-ng/final_pass.slang"), SOURCE).unwrap();
        let preset = root.join("lighting.slangp");
        fs::write(&preset, "#reference \"Base.slangp\"\nGLOBAL_ZOOM = \"1.0\"\nDO_BEZEL = \"0.0\"\nDO_AMBILIGHT = \"0.0\"\n").unwrap();
        install(
            &root.join("Presets-ng/Base.slangp"),
            &preset,
            Path::new("/art/bezel.png"),
            false,
        )
        .unwrap();
        let settings = fs::read_to_string(&preset).unwrap();
        assert_eq!(settings.matches("DO_AMBILIGHT =").count(), 1);
        assert!(settings.contains("DO_AMBILIGHT = \"1.0\""));
        assert_eq!(settings.matches("DO_BEZEL =").count(), 1);
        assert!(settings.contains("GLOBAL_ZOOM = \"1.0\""));
        assert!(settings.contains("BG_IMAGE_WRAP_MODE = \"1.0\""));
        // Koko's auto mode counter-rotates the artwork, not the game. Identity
        // rotates the entire cabinet with vertical arcade content.
        assert!(settings.contains("BG_IMAGE_ROTATION = \"0.0\""));
        assert!(settings.contains("BG_IMAGE_NIGHTIFY = \"0.0\""));
        assert!(settings.contains("shaders = \"18\""));
        assert!(settings.contains("alias16 = \"lunchpail_picture_bounds\""));
        assert!(settings.contains("scale_type16 = \"absolute\""));
        assert!(settings.contains("scale16 = \"4\""));
        assert!(settings.contains("DO_PIXELGRID = \"1.0\""));
        assert!(!settings.contains("#reference"));
        assert!(settings.contains("bg_over = \"/art/bezel.png\""));
        for name in ["reflection_blur_pre.slang", "reflection_blur.slang"] {
            let path = root.join("shaders-ng").join(name);
            assert!(settings.contains(&path.to_string_lossy().replace('\\', "/")));
            assert!(
                fs::read_to_string(path)
                    .unwrap()
                    .contains("if (DO_BEZEL == 0.0) return;")
            );
        }
        assert!(settings.contains(&format!(
            "shader17 = \"{}\"",
            root.join("lighting.slang").to_string_lossy().replace('\\', "/")
        )));
    }
}
