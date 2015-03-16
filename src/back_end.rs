use std::marker::PhantomData;
use gfx;
use gfx::traits::*;
use graphics::{ Context, DrawState, Graphics };
use graphics::BACK_END_MAX_VERTEX_COUNT as BUFFER_SIZE;
use Texture;
use shaders;

const POS_COMPONENTS: usize = 2;
const UV_COMPONENTS: usize = 2;

// Boiler plate for automatic attribute construction.
// Needs to be improved on gfx-rs side.
// For some reason, using ``*_COMPONENT` triggers some macros errors.

#[vertex_format]
struct PositionFormat { pos: [f32; 2] }

#[vertex_format]
struct ColorFormat { color: [f32; 4] }

#[vertex_format]
struct TexCoordsFormat { uv: [f32; 2] }

#[shader_param]
struct Params<R: gfx::Resources> {
    color: [f32; 4],
    _dummy: PhantomData<R>,
}

#[shader_param]
struct ParamsUV<R: gfx::Resources> {
    color: [f32; 4],
    s_texture: gfx::shade::TextureParam<R>,
}

/// The data used for drawing 2D graphics.
pub struct Gfx2d<R: gfx::Resources> {
    buffer_pos: gfx::BufferHandle<R, f32>,
    buffer_uv: gfx::BufferHandle<R, f32>,
    batch: gfx::batch::OwnedBatch<Params<R>>,
    batch_uv: gfx::batch::OwnedBatch<ParamsUV<R>>,
}

impl<R: gfx::Resources> Gfx2d<R> {
    /// Creates a new G2D object.
    pub fn new<D>(device: &mut D) -> Self
        where D: gfx::Device
               + gfx::Factory<R>
    {
        use gfx::traits::*;

        let shader_model = device.get_capabilities().shader_model;

        let vertex = gfx::ShaderSource {
            glsl_120: Some(shaders::VERTEX_SHADER[0]),
            glsl_150: Some(shaders::VERTEX_SHADER[1]),
            .. gfx::ShaderSource::empty()
        };
        let fragment = gfx::ShaderSource {
            glsl_120: Some(shaders::FRAGMENT_SHADER[0]),
            glsl_150: Some(shaders::FRAGMENT_SHADER[1]),
            .. gfx::ShaderSource::empty()
        };

        let program = device.link_program(
            vertex.choose(shader_model).unwrap(),
            fragment.choose(shader_model).unwrap())
            .unwrap();

        let vertex = gfx::ShaderSource {
            glsl_120: Some(shaders::VERTEX_SHADER_UV[0]),
            glsl_150: Some(shaders::VERTEX_SHADER_UV[1]),
            .. gfx::ShaderSource::empty()
        };
        let fragment = gfx::ShaderSource {
            glsl_120: Some(shaders::FRAGMENT_SHADER_UV[0]),
            glsl_150: Some(shaders::FRAGMENT_SHADER_UV[1]),
            .. gfx::ShaderSource::empty()
        };

        let program_uv = device.link_program(
            vertex.choose(shader_model).unwrap(),
            fragment.choose(shader_model).unwrap())
            .unwrap();

        let buffer_pos = device.create_buffer(
            POS_COMPONENTS * BUFFER_SIZE,
            gfx::BufferUsage::Dynamic);
        let buffer_uv = device.create_buffer(
            UV_COMPONENTS * BUFFER_SIZE,
            gfx::BufferUsage::Dynamic);

        let mut mesh = gfx::Mesh::new(BUFFER_SIZE as u32);
        mesh.attributes.extend(gfx::VertexFormat::generate(
            None::<&PositionFormat>,
            buffer_pos.raw().clone()
        ).into_iter());

        // Reuse parameters from `mesh`.
        let mut mesh_uv = mesh.clone();
        mesh_uv.attributes.extend(gfx::VertexFormat::generate(
            None::<&TexCoordsFormat>,
            buffer_uv.raw().clone()
        ).into_iter());

        let params = Params {
            color: [1.0; 4],
            _dummy: PhantomData,
        };
        let mut batch = gfx::batch::OwnedBatch::new(mesh, program, params)
            .unwrap();

        let sampler = device.create_sampler(
            gfx::tex::SamplerInfo::new(
                gfx::tex::FilterMethod::Trilinear,
                gfx::tex::WrapMode::Clamp)
        );

        // Create a dummy texture
        let texture_info = gfx::tex::TextureInfo {
            width: 1,
            height: 1,
            depth: 1,
            levels: 1,
            kind: gfx::tex::TextureKind::Texture2D,
            format: gfx::tex::RGBA8,
        };
        let image_info = texture_info.to_image_info();
        let texture = device.create_texture(texture_info)
            .unwrap();
        device.update_texture(&texture, &image_info,
                &[0x20u8, 0xA0, 0xC0, 0x00])
            .unwrap();
        let params_uv = ParamsUV {
            color: [1.0; 4],
            s_texture: (texture, Some(sampler))
        };
        let mut batch_uv = gfx::batch::OwnedBatch::new(
            mesh_uv, program_uv, params_uv)
            .unwrap();

        // Disable culling.
        batch.state.primitive.method =
            gfx::state::RasterMethod::Fill(gfx::state::CullFace::Nothing);
        batch_uv.state.primitive.method =
            gfx::state::RasterMethod::Fill(gfx::state::CullFace::Nothing);

        Gfx2d {
            buffer_pos: buffer_pos,
            buffer_uv: buffer_uv,
            batch: batch,
            batch_uv: batch_uv,
        }
    }

    /// Renders graphics to a Gfx renderer.
    pub fn draw<C, F>(
        &mut self,
        renderer: &mut gfx::Renderer<R, C>,
        frame: &gfx::Frame<R>,
        mut f: F
    )
        where C: gfx::CommandBuffer<R>,
              F: FnMut(Context, &mut GfxGraphics<R, C>)
    {
        let ref mut g = GfxGraphics::new(
            renderer,
            frame,
            self
        );
        let c = Context::abs(
            frame.width as f64,
            frame.height as f64
        );
        f(c, g);
    }
}

/// Used for rendering 2D graphics.
pub struct GfxGraphics<'a, R, C>
    where R: gfx::Resources + 'a,
          C: gfx::CommandBuffer<R> + 'a,
          R::Buffer: 'a,
          R::ArrayBuffer: 'a,
          R::Shader: 'a,
          R::Program: 'a,
          R::FrameBuffer: 'a,
          R::Surface: 'a,
          R::Texture: 'a,
          R::Sampler: 'a
{
    renderer: &'a mut gfx::Renderer<R, C>,
    frame: &'a gfx::Frame<R>,
    g2d: &'a mut Gfx2d<R>,
}

impl<'a, R, C> GfxGraphics<'a, R, C>
    where R: gfx::Resources,
          C: gfx::CommandBuffer<R>
{
    /// Creates a new object for rendering 2D graphics.
    pub fn new(renderer: &'a mut gfx::Renderer<R, C>,
               frame: &'a gfx::Frame<R>,
               g2d: &'a mut Gfx2d<R>) -> Self {
        GfxGraphics {
            renderer: renderer,
            frame: frame,
            g2d: g2d,
        }
    }

    /// Returns true if texture has alpha channel.
    pub fn has_texture_alpha(&self, texture: &Texture<R>) -> bool {
        use gfx::tex::Components::RGBA;

        texture.handle.get_info().format.get_components() == Some(RGBA)
    }
}

impl<'a, R, C> Graphics for GfxGraphics<'a, R, C>
    where R: gfx::Resources,
          C: gfx::CommandBuffer<R>,
          R::Buffer: 'a,
          R::ArrayBuffer: 'a,
          R::Shader: 'a,
          R::Program: 'a,
          R::FrameBuffer: 'a,
          R::Surface: 'a,
          R::Texture: 'a,
          R::Sampler: 'a
{
    type Texture = Texture<R>;

    fn clear(&mut self, color: [f32; 4]) {
        let &mut GfxGraphics {
            ref mut renderer,
            frame,
            ..
        } = self;
        renderer.clear(
            gfx::ClearData {
                color: color,
                depth: 0.0,
                stencil: 0,
            },
            gfx::COLOR,
            frame
        );
    }

    fn tri_list<F>(
        &mut self,
        draw_state: &DrawState,
        color: &[f32; 4],
        mut f: F
    )
        where F: FnMut(&mut FnMut(&[f32]))
    {
        let &mut GfxGraphics {
            ref mut renderer,
            ref frame,
            g2d: &mut Gfx2d {
                ref mut buffer_pos,
                ref mut batch,
                ..
            },
        } = self;

        batch.state = *draw_state;
        batch.param.color = *color;

        f(&mut |vertices: &[f32]| {
            renderer.update_buffer_vec(&buffer_pos, vertices, 0);

            let n = vertices.len() / POS_COMPONENTS;
            batch.slice = gfx::Slice {
                    prim_type: gfx::PrimitiveType::TriangleList,
                    start: 0,
                    end: n as u32,
                    kind: gfx::SliceKind::Vertex
                };
            let _ = renderer.draw(batch, *frame);
        })
    }

    fn tri_list_uv<F>(
        &mut self,
        draw_state: &DrawState,
        color: &[f32; 4],
        texture: &<Self as Graphics>::Texture,
        mut f: F
    )
        where F: FnMut(&mut FnMut(&[f32], &[f32]))
    {
        let &mut GfxGraphics {
            ref mut renderer,
            ref frame,
            g2d: &mut Gfx2d {
                ref mut buffer_pos,
                ref mut buffer_uv,
                ref mut batch_uv,
                ..
            },
        } = self;

        batch_uv.state = *draw_state;
        batch_uv.param.s_texture.0 = texture.handle.clone();
        batch_uv.param.color = *color;

        f(&mut |vertices: &[f32], texture_coords: &[f32]| {
            assert_eq!(
                vertices.len() * UV_COMPONENTS,
                texture_coords.len() * POS_COMPONENTS
            );
            renderer.update_buffer_vec(&buffer_pos, vertices, 0);
            renderer.update_buffer_vec(&buffer_uv, texture_coords, 0);

            let n = vertices.len() / POS_COMPONENTS;
            batch_uv.slice = gfx::Slice {
                    prim_type: gfx::PrimitiveType::TriangleList,
                    start: 0,
                    end: n as u32,
                    kind: gfx::SliceKind::Vertex
                };
            let _ = renderer.draw(batch_uv, *frame);
        })
    }
}