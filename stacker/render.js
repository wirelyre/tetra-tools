import * as twgl from "./twgl/twgl-full.module.js";

const fourBoardVertex = `
#version 300 es

#define WIDTH  10
#define HEIGHT  9

uniform lowp usampler2D u_board;
uniform mat4 u_matrix;

in vec2 a_pos;
in vec2 a_texCoord;

out vec2 v_pos;
out vec2 v_texCoord;

uint getKind(int idx) {
    return texelFetch(u_board, ivec2(idx, 0), 0).x;
}

uint getInfo(int idx) {
    return texelFetch(u_board, ivec2(idx, 0), 0).y;
}

void draw(int idx, int pass);

void main() {
    int count = WIDTH * HEIGHT;
    draw(gl_InstanceID % count, gl_InstanceID / count);
}

void draw(int idx, int pass) {
    int col = idx % WIDTH;
    int row = idx / WIDTH;

    if (pass == 0) {
        if (getKind(idx) == uint(0)) {
            // discard triangle
            gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        } else {
            // draw shadow
            v_texCoord = a_pos * vec2(20);
            v_texCoord.x = 177.0 + v_texCoord.x;
            v_texCoord.y = 32.0 - v_texCoord.y;
            v_texCoord /= vec2(256, 32);

            v_pos = a_pos + vec2(col, row) + vec2(0.25, -7.0/20.0);
            gl_Position = u_matrix * vec4(v_pos, 0, 1);
        }
    } else {
        uint kind = getKind(idx);
        vec2 sprite = vec2(min(kind, uint(9)), 0);

        v_texCoord = a_pos * vec2(21, 24) + sprite * vec2(22, 24);
        v_texCoord.y = 32.0 - v_texCoord.y;
        v_texCoord /= vec2(256, 32);

        v_pos = a_pos * vec2(1, 24.0 / 20.0) + vec2(col, row);
        gl_Position = u_matrix * vec4(v_pos, 0, 1);
    }
}
`;

const fourFragment = `
#version 300 es

precision mediump float;

uniform sampler2D u_tex;

in vec2 v_pos;
in vec2 v_texCoord;

out vec4 color;

void main() {
    color = texture(u_tex, v_texCoord);
}
`;

class Renderer {
    constructor(gl) {
        if (!twgl.isWebGL2(gl)) {
            throw "need WebGL2";
        }

        this.gl = gl;
        this.programInfo = twgl.createProgramInfo(gl, [fourBoardVertex, fourFragment]);

        gl.clearColor(0xF3 / 0xFF, 0xF3 / 0xFF, 0xED / 0xFF, 1);
        gl.clear(gl.COLOR_BUFFER_BIT);
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    }

    async init() {
        const triangles = {
            a_pos: {
                numComponents: 2, data: [
                    0, 0, 1, 0, 1, 1,
                    0, 0, 0, 1, 1, 1,
                ]
            },
        };
        this.bufferInfo = twgl.createBufferInfoFromArrays(this.gl, triangles);

        await new Promise(resolve => {
            this.textures = twgl.createTextures(this.gl, {
                atlas: {
                    src: "four.png",
                    width: 256,
                    height: 32,
                    minMag: this.gl.NEAREST,
                },
                board: { internalFormat: this.gl.RG8UI },
            }, resolve);
        });
    }

    draw(board) {
        twgl.setTextureFromArray(this.gl, this.textures.board, board, {
            height: 1,
            width: board.length >> 1,
            format: this.gl.RG_INTEGER,
            internalFormat: this.gl.RG8UI,
            minMag: this.gl.NEAREST,
        });

        const uniforms = {
            u_tex: this.textures.atlas,
            u_board: this.textures.board,
            u_matrix: twgl.m4.scale(twgl.m4.translation([-1, -1, 0]), [1 / 5, 2 / 9, 1]),
        };

        twgl.resizeCanvasToDisplaySize(this.gl.canvas, devicePixelRatio);
        this.gl.viewport(0, 0, this.gl.canvas.width, this.gl.canvas.height);

        this.gl.useProgram(this.programInfo.program);
        this.gl.clear(this.gl.COLOR_BUFFER_BIT);

        twgl.setBuffersAndAttributes(this.gl, this.programInfo, this.bufferInfo);
        twgl.setUniforms(this.programInfo, uniforms);
        this.gl.drawArraysInstanced(this.gl.TRIANGLES, 0, this.bufferInfo.numElements, /* instanceCount */ 10 * 9 * 2);
    }
}

export { Renderer };