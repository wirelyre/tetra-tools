use js_sys::{Array, Uint8Array};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Blob, HtmlCanvasElement, ImageBitmap, WebGl2RenderingContext as Ctx, WebGlShader};
use web_sys::{
    WebGlBuffer, WebGlProgram, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

use crate::{Game, Piece};

#[wasm_bindgen]
pub struct Renderer {
    ctx: Ctx,

    field: FieldRenderer,
    piece: PieceRenderer,
    atlas: WebGlTexture,
}

// TODO: impl Drop
struct FieldRenderer {
    vertex_array: WebGlVertexArrayObject,
    texture: WebGlTexture,
    texture_size: i32,

    program: WebGlProgram,
    u_field_location: Option<WebGlUniformLocation>,
    u_width_location: Option<WebGlUniformLocation>,
    u_height_location: Option<WebGlUniformLocation>,
    u_matrix_location: Option<WebGlUniformLocation>,
    u_tex_location: Option<WebGlUniformLocation>,
}

// TODO: impl Drop
struct PieceRenderer {
    vertex_array: WebGlVertexArrayObject,
    buffer_minoes: WebGlBuffer,

    program: WebGlProgram,
    u_matrix_location: Option<WebGlUniformLocation>,
    u_mino_color_location: Option<WebGlUniformLocation>,
    u_tex_location: Option<WebGlUniformLocation>,
}

fn create_program(ctx: &Ctx, vs: &WebGlShader, fs: &WebGlShader) -> WebGlProgram {
    let program = ctx.create_program().unwrap();
    ctx.attach_shader(&program, &vs);
    ctx.attach_shader(&program, &fs);
    ctx.link_program(&program);

    if ctx
        .get_program_parameter(&program, Ctx::LINK_STATUS)
        .as_bool()
        != Some(true)
    {
        panic!(
            "program error\n\ninfo log: {}\n\nvertex shader: {}\n\nfragment shader: {}",
            ctx.get_program_info_log(&program)
                .unwrap_or_else(|| "okay".to_string()),
            ctx.get_shader_info_log(&vs)
                .unwrap_or_else(|| "okay".to_string()),
            ctx.get_shader_info_log(&fs)
                .unwrap_or_else(|| "okay".to_string()),
        );
    }

    program
}

impl FieldRenderer {
    fn new(
        ctx: &Ctx,
        triangles: &WebGlBuffer,
        min_size: i32,
        vs: &WebGlShader,
        fs: &WebGlShader,
    ) -> FieldRenderer {
        let vertex_array = ctx.create_vertex_array().unwrap();
        ctx.bind_vertex_array(Some(&vertex_array));
        ctx.bind_buffer(Ctx::ARRAY_BUFFER, Some(&triangles));
        ctx.enable_vertex_attrib_array(0);
        ctx.vertex_attrib_pointer_with_i32(0, 2, Ctx::UNSIGNED_BYTE, false, 0, 0);
        ctx.bind_vertex_array(None);

        let texture = FieldRenderer::create_texture(&ctx, min_size);
        let texture_size = min_size;

        let program = create_program(ctx, vs, fs);
        let u_field_location = ctx.get_uniform_location(&program, "u_field");
        let u_width_location = ctx.get_uniform_location(&program, "u_width");
        let u_height_location = ctx.get_uniform_location(&program, "u_height");
        let u_matrix_location = ctx.get_uniform_location(&program, "u_matrix");
        let u_tex_location = ctx.get_uniform_location(&program, "u_tex");

        FieldRenderer {
            vertex_array,
            texture,
            texture_size,
            program,
            u_field_location,
            u_width_location,
            u_height_location,
            u_matrix_location,
            u_tex_location,
        }
    }

    fn create_texture(ctx: &Ctx, size: i32) -> WebGlTexture {
        let texture = ctx.create_texture().unwrap();
        ctx.bind_texture(Ctx::TEXTURE_2D, Some(&texture));
        ctx.tex_parameteri(
            Ctx::TEXTURE_2D,
            Ctx::TEXTURE_MAG_FILTER,
            Ctx::NEAREST as i32,
        );
        ctx.tex_parameteri(
            Ctx::TEXTURE_2D,
            Ctx::TEXTURE_MIN_FILTER,
            Ctx::NEAREST as i32,
        );
        ctx.tex_storage_2d(Ctx::TEXTURE_2D, 1, Ctx::RG8UI, size as i32, 1);
        texture
    }

    fn upload_to_texture(&mut self, ctx: &Ctx, data: &[u8]) {
        ctx.bind_texture(Ctx::TEXTURE_2D, Some(&self.texture));

        let count = (data.len() as i32 + 1) / 2;
        if self.texture_size < count {
            self.texture_size = count;
            ctx.delete_texture(Some(&self.texture));
            self.texture = FieldRenderer::create_texture(&ctx, count);
        }

        ctx.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
            Ctx::TEXTURE_2D,
            0,
            0,
            0,
            count as i32,
            1,
            Ctx::RG_INTEGER,
            Ctx::UNSIGNED_BYTE,
            Some(data),
        )
        .unwrap();
    }

    fn render(
        &mut self,
        ctx: &Ctx,
        data: &[u8],
        u_width: u8,
        u_height: u8,
        u_matrix: &[f32],
        atlas: &WebGlTexture,
    ) {
        ctx.use_program(Some(&self.program));

        ctx.active_texture(Ctx::TEXTURE0);
        self.upload_to_texture(ctx, data);
        ctx.uniform1i(self.u_field_location.as_ref(), 0);

        ctx.active_texture(Ctx::TEXTURE1);
        ctx.bind_texture(Ctx::TEXTURE_2D, Some(atlas));
        ctx.uniform1i(self.u_tex_location.as_ref(), 1);

        ctx.uniform1ui(self.u_width_location.as_ref(), u_width as u32);
        ctx.uniform1ui(self.u_height_location.as_ref(), u_height as u32);

        ctx.bind_vertex_array(Some(&self.vertex_array));
        ctx.uniform_matrix4fv_with_f32_array(self.u_matrix_location.as_ref(), false, u_matrix);

        ctx.draw_arrays_instanced(Ctx::TRIANGLES, 0, 6, u_width as i32 * u_height as i32 * 2);
        ctx.bind_vertex_array(None);
        ctx.use_program(None);
    }
}

impl PieceRenderer {
    pub fn new(
        ctx: &Ctx,
        triangles: &WebGlBuffer,
        vs: &WebGlShader,
        fs: &WebGlShader,
    ) -> PieceRenderer {
        let vertex_array = ctx.create_vertex_array().unwrap();
        ctx.bind_vertex_array(Some(&vertex_array));

        ctx.bind_buffer(Ctx::ARRAY_BUFFER, Some(&triangles));
        ctx.enable_vertex_attrib_array(0);
        ctx.vertex_attrib_pointer_with_i32(0, 2, Ctx::UNSIGNED_BYTE, false, 0, 0);

        let buffer_minoes = ctx.create_buffer().unwrap();
        ctx.bind_buffer(Ctx::ARRAY_BUFFER, Some(&buffer_minoes));
        ctx.enable_vertex_attrib_array(1);
        ctx.vertex_attrib_pointer_with_i32(1, 2, Ctx::UNSIGNED_BYTE, false, 0, 0);
        ctx.vertex_attrib_divisor(1, 1);

        ctx.bind_vertex_array(None);

        let program = create_program(ctx, vs, fs);
        let u_matrix_location = ctx.get_uniform_location(&program, "u_matrix");
        let u_mino_color_location = ctx.get_uniform_location(&program, "u_mino_color");
        let u_tex_location = ctx.get_uniform_location(&program, "u_tex");

        PieceRenderer {
            vertex_array,
            buffer_minoes,
            program,
            u_matrix_location,
            u_mino_color_location,
            u_tex_location,
        }
    }

    pub fn render(
        &self,
        ctx: &Ctx,
        game: &Game,
        piece: &Piece,
        u_matrix: &[f32],
        atlas: &WebGlTexture,
    ) {
        ctx.use_program(Some(&self.program));

        ctx.active_texture(Ctx::TEXTURE1);
        ctx.bind_texture(Ctx::TEXTURE_2D, Some(atlas));
        ctx.uniform1i(self.u_tex_location.as_ref(), 1);

        ctx.bind_vertex_array(Some(&self.vertex_array));
        ctx.uniform_matrix4fv_with_f32_array(self.u_matrix_location.as_ref(), false, u_matrix);

        ctx.uniform1ui(
            self.u_mino_color_location.as_ref(),
            piece.color(game) as u32,
        );

        ctx.bind_buffer(Ctx::ARRAY_BUFFER, Some(&self.buffer_minoes));
        ctx.buffer_data_with_u8_array(Ctx::ARRAY_BUFFER, &piece.minoes(game), Ctx::DYNAMIC_DRAW);

        ctx.draw_arrays_instanced(Ctx::TRIANGLES, 0, 6, 4);
        ctx.bind_vertex_array(None);
        ctx.use_program(None);
    }
}

#[wasm_bindgen]
impl Renderer {
    #[wasm_bindgen(constructor)]
    pub async fn new(ctx: Ctx) -> Renderer {
        console_error_panic_hook::set_once();

        assert!(Ctx::instanceof(&ctx), "need WebGL2 context");

        ctx.enable(Ctx::BLEND);
        ctx.blend_func(Ctx::SRC_ALPHA, Ctx::ONE_MINUS_SRC_ALPHA);

        let triangles = ctx.create_buffer().unwrap();
        ctx.bind_buffer(Ctx::ARRAY_BUFFER, Some(&triangles));
        ctx.buffer_data_with_u8_array(
            Ctx::ARRAY_BUFFER,
            &[0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1],
            Ctx::STATIC_DRAW,
        );

        let vs = ctx.create_shader(Ctx::VERTEX_SHADER).unwrap();
        ctx.shader_source(&vs, FOUR_FIELD_VS);
        ctx.compile_shader(&vs);
        let fs = ctx.create_shader(Ctx::FRAGMENT_SHADER).unwrap();
        ctx.shader_source(&fs, FOUR_FIELD_FS);
        ctx.compile_shader(&fs);

        let field_renderer = FieldRenderer::new(&ctx, &triangles, 1, &vs, &fs);

        ctx.delete_shader(Some(&vs));
        ctx.delete_shader(Some(&fs));

        let vs = ctx.create_shader(Ctx::VERTEX_SHADER).unwrap();
        ctx.shader_source(&vs, FOUR_PIECE_VS);
        ctx.compile_shader(&vs);
        let fs = ctx.create_shader(Ctx::FRAGMENT_SHADER).unwrap();
        ctx.shader_source(&fs, FOUR_PIECE_FS);
        ctx.compile_shader(&fs);

        let piece_renderer = PieceRenderer::new(&ctx, &triangles, &vs, &fs);

        ctx.delete_shader(Some(&vs));
        ctx.delete_shader(Some(&fs));

        let blob = unsafe {
            Blob::new_with_u8_array_sequence(
                Array::of1(Uint8Array::view(FOUR_ATLAS).as_ref()).as_ref(),
            )
        }
        .unwrap();
        #[allow(unused_unsafe)]
        let bitmap = unsafe { create_image_bitmap(blob) }
            .await
            .dyn_into::<ImageBitmap>()
            .unwrap();
        let atlas = ctx.create_texture().unwrap();
        ctx.bind_texture(Ctx::TEXTURE_2D, Some(&atlas));
        ctx.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_image_bitmap(
            Ctx::TEXTURE_2D,
            0,
            Ctx::RGBA as i32,
            256,
            32,
            0,
            Ctx::RGBA,
            Ctx::UNSIGNED_BYTE,
            &bitmap,
        )
        .unwrap();
        ctx.generate_mipmap(Ctx::TEXTURE_2D);

        Renderer {
            ctx,
            field: field_renderer,
            piece: piece_renderer,
            atlas,
        }
    }

    #[wasm_bindgen(js_name = drawField)]
    pub fn draw_field(&mut self, game: &Game) {
        self.ctx
            .clear_color(243. / 255., 243. / 255., 237. / 255., 1.0);
        self.ctx.clear(Ctx::COLOR_BUFFER_BIT);

        let u_matrix = [
            1. / 5.,
            0.,
            0.,
            0.,
            0.,
            2. / 9.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            -1.,
            -1.,
            0.,
            1.,
        ];
        self.field.render(
            &self.ctx,
            game.get_field(),
            game.width,
            game.height,
            &u_matrix,
            &self.atlas,
        );
    }

    #[wasm_bindgen(js_name = drawPiece)]
    pub fn draw_piece(&mut self, game: &Game, piece: &Piece) {
        let u_matrix = [
            1. / 5.,
            0.,
            0.,
            0.,
            0.,
            2. / 9.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            -1.,
            -1.,
            0.,
            1.,
        ];
        self.piece
            .render(&self.ctx, game, piece, &u_matrix, &self.atlas);
    }

    pub fn fix_pixel_size(&self) {
        let canvas = self
            .ctx
            .canvas()
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        let multiplier = web_sys::window().unwrap().device_pixel_ratio();
        let width = canvas.client_width() as f64 * multiplier;
        let height = canvas.client_height() as f64 * multiplier;
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
    }
}

static FOUR_FIELD_VS: &str = r#"#version 300 es
    uniform uint u_width;
    uniform uint u_height;
    uniform lowp usampler2D u_field;
    uniform mat4 u_matrix;

    layout(location = 0) in vec2 a_pos;

    out vec2 v_texCoord;

    uint getKind(int idx) {
        return texelFetch(u_field, ivec2(idx, 0), 0).x;
    }

    uint getInfo(int idx) {
        return texelFetch(u_field, ivec2(idx, 0), 0).y;
    }

    void draw(int idx, int pass);

    void main() {
        int count = int(u_width * u_height); // WIDTH * HEIGHT;
        draw(gl_InstanceID % count, gl_InstanceID / count);
    }

    void draw(int idx, int pass) {
        int col = idx % int(u_width); // WIDTH;
        int row = idx / int(u_width); // WIDTH;

        if (pass == 0) {
            if (getKind(idx) == uint(0)) {
                // discard triangle
                gl_Position = vec4(2, 2, 2, 1);
            } else {
                // draw shadow
                v_texCoord = a_pos * vec2(20);
                v_texCoord.x = 177.0 + v_texCoord.x;
                v_texCoord.y = 32.0 - v_texCoord.y;
                v_texCoord /= vec2(256, 32);

                vec2 v_pos = a_pos + vec2(col, row) + vec2(0.25, -7.0/20.0);
                gl_Position = u_matrix * vec4(v_pos, 0, 1);
            }
        } else {
            uint kind = getKind(idx);
            vec2 sprite = vec2(min(kind, uint(9)), 0);

            v_texCoord = a_pos * vec2(21, 24) + sprite * vec2(22, 24);
            v_texCoord.y = 32.0 - v_texCoord.y;
            v_texCoord /= vec2(256, 32);

            vec2 v_pos = a_pos * vec2(1, 24.0 / 20.0) + vec2(col, row);
            gl_Position = u_matrix * vec4(v_pos, 0, 1);
        }
    }
"#;
static FOUR_FIELD_FS: &str = r#"#version 300 es
    precision mediump float;
    uniform sampler2D u_tex;
    in vec2 v_pos;
    in vec2 v_texCoord;
    layout(location = 0) out vec4 color;
    void main() {
        color = texture(u_tex, v_texCoord);
    }
"#;
static FOUR_PIECE_VS: &str = r#"#version 300 es
    uniform mat4 u_matrix;
    uniform uint u_mino_color;

    layout(location = 0) in vec2 a_pos;
    layout(location = 1) in vec2 a_coords;

    out vec2 v_texCoord;

    void main() {
        gl_Position = u_matrix * vec4(a_pos + a_coords, 0, 1);

        v_texCoord = a_pos * vec2(19, 19) + vec2(u_mino_color * uint(22), 0);
        v_texCoord.y = 32.0 - v_texCoord.y;
        v_texCoord /= vec2(256, 32);
    }
"#;
static FOUR_PIECE_FS: &str = r#"#version 300 es
    precision mediump float;
    uniform sampler2D u_tex;
    in vec2 v_texCoord;
    layout(location = 0) out vec4 color;
    void main() {
        color = texture(u_tex, v_texCoord);
    }
"#;

static FOUR_ATLAS: &[u8] = include_bytes!("../four.png");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = createImageBitmap)]
    async fn create_image_bitmap(image: Blob) -> JsValue;
}
