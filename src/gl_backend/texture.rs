pub struct GLTexture {
    texture_handle: u32,
}

impl GLTexture {
    pub fn new(
        image_data: &[u8],
        dimensions: (u32, u32),
        min_filter: u32,
        mag_filter: u32,
        texture_type: u32,
    ) -> GLTexture {
        let mut texture_handle = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_handle);
            gl::BindTexture(gl::TEXTURE_2D, texture_handle);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min_filter as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag_filter as _);

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as _,
                dimensions.0 as _,
                dimensions.1 as _,
                0,
                texture_type,
                gl::UNSIGNED_BYTE,
                image_data.as_ptr() as _,
            );

            // gl::TexStorage2D(
            //     gl::TEXTURE_2D,
            //     9,
            //     gl::RGBA8 as _,
            //     dimensions.0 as _,
            //     dimensions.1 as _,
            // );

            // gl::TexSubImage2D(
            //     gl::TEXTURE_2D,
            //     0,
            //     0,
            //     0,
            //     dimensions.0 as _,
            //     dimensions.1 as _,
            //     texture_type,
            //     gl::UNSIGNED_BYTE,
            //     image_data.as_ptr() as _,
            // )
        }

        GLTexture { texture_handle }
    }

    pub fn bind(&self, idx: i32) {
        let bind_location = match idx {
            0 => gl::TEXTURE0,
            1 => gl::TEXTURE1,
            2 => gl::TEXTURE2,
            _ => panic!("invalid idx"),
        };

        unsafe {
            gl::ActiveTexture(bind_location);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_handle);
        }
    }
}
