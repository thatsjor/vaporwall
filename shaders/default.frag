#version 330 core

out vec4 FragColor;

uniform float u_time;
uniform vec2 u_resolution;
uniform vec3 u_background;
uniform vec3 u_color1;
uniform vec3 u_color2;
uniform vec3 u_color3;
uniform vec3 u_color4;

void main() {
    vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution.xy) / u_resolution.y;
    
    // Background glow
    vec3 color = u_background * 0.5;
    
    // Horizon
    float horizon = -0.2;
    float dist = abs(uv.y - horizon);
    
    if (uv.y < horizon) {
        // Ground Grid
        float perspective = 1.0 / (horizon - uv.y);
        vec2 grid_uv = vec2(uv.x * perspective, perspective + u_time * 0.5);
        
        // Grid lines
        vec2 grid = abs(fract(grid_uv * 10.0 - 0.5) - 0.5) / fwidth(grid_uv * 10.0);
        float line = min(grid.x, grid.y);
        float grid_pattern = 1.0 - smoothstep(0.0, 0.05, line);
        
        // Fade grid into horizon
        float fade = smoothstep(0.0, 4.0, perspective);
        color = mix(color, u_color1, grid_pattern * fade);
        
        // Horizontal scanlines for atmosphere
        color += u_color2 * 0.1 * sin(uv.y * 100.0 + u_time * 2.0);
    } else {
        // Sky
        // Simple stars
        vec2 star_uv = uv * 20.0;
        vec2 ipos = floor(star_uv);
        vec2 fpos = fract(star_uv);
        float rand = fract(sin(dot(ipos, vec2(12.9898, 78.233))) * 43758.5453);
        if (rand > 0.98) {
            float blink = 0.5 + 0.5 * sin(u_time * 3.0 + rand * 10.0);
            float dist_star = length(fpos - 0.5);
            color += u_color3 * (1.0 - smoothstep(0.0, 0.1, dist_star)) * blink;
        }
        
        // Sky gradient
        color += u_color2 * 0.1 * (1.0 - uv.y);
    }

    // Vignette
    float vignette = 1.0 - length(uv * 0.5);
    color *= vignette;

    FragColor = vec4(color, 1.0);
}
