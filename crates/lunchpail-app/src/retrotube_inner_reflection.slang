// Fragment snippet inserted into Koko's final compositor. Its original
// reflection_blur_pre/reflection_blur passes and fn_pixel_bezel provide the
// material controls, aspect-correct blur, corner shading and roughness model.
// External artwork has no material LUT. Include both transparent padding and
// the dark, opaque bevel immediately beside the picture; many console games
// fill their aperture and have no transparent padding. Keep the printed outer
// artwork untouched and never alter the picture's size/curvature.
vec3 light = vec3(0.0);
float lunchpailArtBounds = step(0.0, image_coords.x) * step(image_coords.x, 1.0)
    * step(0.0, image_coords.y) * step(image_coords.y, 1.0);
vec2 lunchpailEdge = clamp(co_content, lunchpailPictureBounds.xy, lunchpailPictureBounds.zw);
vec2 lunchpailOutside = abs(co_content - lunchpailEdge);
lunchpailOutside.x *= vIn_aspect;
float lunchpailDistance = length(lunchpailOutside);
float lunchpailPadding = (1.0 - smoothstep(0.0, 0.10, lunchpailDistance))
    * (1.0 - pixel_fg_image.a);
// The material band is narrower than the possible native padding. Its
// distance is measured from the curved picture edge, not the image rectangle,
// so the light stops before the TV's outer frame, logos and printed cabinet.
float lunchpailArtBrightness = max(max(pixel_fg_image.r, pixel_fg_image.g), pixel_fg_image.b);
float lunchpailBevel = (1.0 - smoothstep(0.0, 0.055, lunchpailDistance))
    * (1.0 - smoothstep(0.10, 0.28, lunchpailArtBrightness)) * pixel_fg_image.a;
float lunchpailLip = step(0.00001, lunchpailDistance)
    * (lunchpailPadding + lunchpailBevel) * lunchpailArtBounds;
if (lunchpailLip > 0.0 && BEZEL_REFL_STRENGTH > 0.0) {
    // Like Koko's mirrored-repeat sampler: flip about the curved picture edge,
    // not the flat artwork rectangle. Normalize only the material coordinates
    // so native black padding does not change the reflection's direction.
    vec2 lunchpailSize = lunchpailPictureBounds.zw - lunchpailPictureBounds.xy;
    vec2 lunchpailSurface = (co_content - lunchpailPictureBounds.xy) / lunchpailSize;
    vec2 lunchpailMirror = 2.0 * lunchpailEdge - co_content;
    vec2 lunchpailMirrorUnit = (lunchpailMirror - lunchpailPictureBounds.xy) / lunchpailSize;
    lunchpailMirrorUnit = zoomout_coords(lunchpailMirrorUnit, BEZEL_RFL_ZOOM * 0.1);

    // Koko increases blur away from the edge and at the corners. Combine the
    // two original blur radii in quadrature for one bounded eight-tap gather;
    // this avoids adding two full-frame passes for a narrow strip of material.
    vec2 lunchpailGradient = 1.0 - smoothstep_fast(vec2(0.35), vec2(0.5),
        abs(zoom(lunchpailMirrorUnit, 1.05) - 0.5));
    float lunchpailSharpBlur = clamp(lunchpailGradient.x * lunchpailGradient.y * 2.5
        - 0.5 - BEZEL_RFL_BLR_SHD, 0.0, 1.0);
    float lunchpailPreRadius = lunchpailSharpBlur * 2.0;
    float lunchpailFinalRadius = max(1.0, max(lunchpailSharpBlur,
        corners_shade(lunchpailSurface, 0.9) * 0.6 * 0.9) * 3.5);
    vec2 lunchpailBlur = radius_blur_adapt_ext_inaspect(
        length(vec2(lunchpailPreRadius, lunchpailFinalRadius)), vIn_aspect);

    // Static micro-roughness, as in Koko. Never animate this noise: shimmering
    // highlights would be distracting and invalidate unchanged-frame reuse.
    float lunchpailRoughness = (random_fast(vTexCoord) - 0.5) * (2.0 / 1080.0) * BEZEL_ROUGHNESS;
    vec3 lunchpailReflection = vec3(0.0);
    for (int direction = 0; direction < 8; ++direction) {
        float angle = (float(direction) + 0.25) * (2.0 * pi / 8.0);
        vec2 lunchpailSample = lunchpailPictureBounds.xy + lunchpailSize
            * (lunchpailMirrorUnit + lunchpailRoughness + vec2(cos(angle), sin(angle)) * lunchpailBlur);
        lunchpailSample = clamp(lunchpailSample, lunchpailPictureBounds.xy, lunchpailPictureBounds.zw);
        lunchpailReflection += texture(in_glow_pass, lunchpailSample).rgb * 0.125;
    }
    lunchpailReflection = pow(max(lunchpailReflection, vec3(0.0)), vec3(GAMMA_OUT));
    float lunchpailCorners = 1.0 - corners_shade(lunchpailSurface, 1.125) * BEZEL_CORNER_DARK;
    // Our source is already the glowing picture, not Koko's pre-glow source.
    // Apply strength in display space: RetroTube's 0.33 replaces the previous
    // hard-coded 0.55, without gamma lifting the dark material back to a mirror.
    pixel_out += lunchpailReflection * lunchpailLip * lunchpailCorners * BEZEL_REFL_STRENGTH;
}
