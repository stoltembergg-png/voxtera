#ifndef SHADOWS_GLSL
#define SHADOWS_GLSL

#ifdef HAS_SHADOW_MAPS
    #if (SHADOW_MODE == SHADOW_MODE_MAP)
        layout (std140, set = 0, binding = 9)
        uniform u_light_shadows {
            mat4 shadowMatrices;
            mat4 texture_mat;
        };
        
        // Use with sampler2DShadow
        layout(set = 1, binding = 2)
        uniform texture2D t_directed_shadow_maps;
        layout(set = 1, binding = 3)
        uniform samplerShadow s_directed_shadow_maps;
        
        // Use with samplerCubeShadow
        layout(set = 1, binding = 0)
        uniform textureCube t_point_shadow_maps;
        layout(set = 1, binding = 1)
        uniform samplerShadow s_point_shadow_maps;
        
        float VectorToDepth(vec3 Vec) {
            vec3 AbsVec = abs(Vec);
            float LocalZcomp = max(AbsVec.x, max(AbsVec.y, AbsVec.z));
        
            float NormZComp = shadow_proj_factors.x - shadow_proj_factors.y / LocalZcomp;
            return NormZComp;
        }
        
        const vec3 sampleOffsetDirections[20] = vec3[](
            vec3( 1,  1,  1), vec3( 1, -1,  1), vec3(-1, -1,  1), vec3(-1,  1,  1),
            vec3( 1,  1, -1), vec3( 1, -1, -1), vec3(-1, -1, -1), vec3(-1,  1, -1),
            vec3( 1,  1,  0), vec3( 1, -1,  0), vec3(-1, -1,  0), vec3(-1,  1,  0),
            vec3( 1,  0,  1), vec3(-1,  0,  1), vec3( 1,  0, -1), vec3(-1,  0, -1),
            vec3( 0,  1,  1), vec3( 0, -1,  1), vec3( 0, -1, -1), vec3( 0,  1, -1)
        );
        
        float ShadowCalculationPoint(uint lightIndex, vec3 fragToLight, vec3 fragNorm, vec3 fragPos) {
            if (lightIndex != 0u) {
                return 1.0;
            };
        
            float currentDepth = VectorToDepth(fragToLight);
        
            return textureGrad(samplerCubeShadow(t_point_shadow_maps, s_point_shadow_maps), vec4(fragToLight, currentDepth), vec3(0), vec3(0));
        }
        
        float ShadowCalculationDirected(in vec3 fragPos) {
            // Don't try to calculate directed shadows if there are no directed light sources
            // Applies, for example, in the char select menu
            if (light_shadow_count.z < 1) { return 1.0; }

            // PCF 3x3 filtering for smoother shadow edges
            // Samples the shadow map at 9 locations around the center and averages
            float bias = 0.0;
            float diskRadius = 0.01;
            vec4 sun_pos = texture_mat * vec4(fragPos, 1.0);

            // Perform PCF by sampling the shadow map at 9 offsets
            vec2 shadow_map_size = vec2(textureSize(sampler2DShadow(t_directed_shadow_maps, s_directed_shadow_maps), 0));
            vec2 texel_size = 1.0 / shadow_map_size;

            float shadow = 0.0;
            float current_depth = sun_pos.z / sun_pos.w;

            // Early-out if fragment is outside the shadow frustum
            if (sun_pos.w <= 0.0 ||
                current_depth > 1.0 || current_depth < 0.0) {
                return 1.0;
            }

            // 3x3 PCF kernel
            for (int x = -1; x <= 1; x++) {
                for (int y = -1; y <= 1; y++) {
                    vec2 offset = vec2(x, y) * texel_size;
                    vec4 sample_pos = vec4(sun_pos.xy + offset * sun_pos.w, sun_pos.z, sun_pos.w);
                    shadow += textureProj(sampler2DShadow(t_directed_shadow_maps, s_directed_shadow_maps), sample_pos);
                }
            }
            shadow /= 9.0;

            return shadow;
        }
    #elif (SHADOW_MODE == SHADOW_MODE_NONE || SHADOW_MODE == SHADOW_MODE_CHEAP)
        float ShadowCalculationPoint(uint lightIndex, vec3 fragToLight, vec3 fragNorm, vec3 fragPos) {
            return 1.0;
        }
    #endif
#else
    float ShadowCalculationPoint(uint lightIndex, vec3 fragToLight, vec3 fragNorm, vec3 fragPos) {
        return 1.0;
    }
#endif

#endif
