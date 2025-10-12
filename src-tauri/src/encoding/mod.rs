// Video encoding module using FFmpeg

use ffmpeg_next as ffmpeg;
use std::path::PathBuf;

pub struct VideoEncoder {
    width: u32,
    height: u32,
    fps: u32,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        Self { width, height, fps }
    }

    /// Decode PNG frames to raw RGB data
    fn decode_frames(&self, frames: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, String> {
        println!("🔄 Decoding {} PNG frames to RGB...", frames.len());
        
        frames
            .iter()
            .enumerate()
            .map(|(i, png_bytes)| {
                let img = image::load_from_memory(png_bytes)
                    .map_err(|e| format!("Failed to decode frame {}: {}", i, e))?;
                
                if i == 0 {
                    println!("📊 Frame 0 - Original format: {:?}, Dimensions: {}x{}", 
                        img.color(), img.width(), img.height());
                }
                
                // Convert to RGB8 (ensures proper color format)
                let rgb_img = img.to_rgb8();
                
                if i == 0 {
                    println!("📊 Frame 0 - After conversion: {}x{}, Data size: {} bytes, Bytes per pixel: {}", 
                        rgb_img.width(), rgb_img.height(), rgb_img.len(),
                        rgb_img.len() / (rgb_img.width() * rgb_img.height()) as usize);
                }
                
                Ok(rgb_img.into_raw())
            })
            .collect()
    }

    /// Encode frames to MP4 video file
    pub fn encode_to_mp4(
        &self,
        frames: Vec<Vec<u8>>,
        output_path: PathBuf,
    ) -> Result<(), String> {
        if frames.is_empty() {
            return Err("No frames to encode".to_string());
        }

        println!("🎬 Encoding {} frames to MP4...", frames.len());
        println!("📐 Resolution: {}×{} @ {} fps", self.width, self.height, self.fps);

        // Initialize FFmpeg (safe to call multiple times)
        ffmpeg::init().map_err(|e| format!("FFmpeg init failed: {}", e))?;

        // Decode PNG frames to raw RGB
        let raw_frames = self.decode_frames(&frames)?;

        // Create output context
        let mut octx = ffmpeg::format::output(&output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;

        // Find H.264 encoder
        let encoder_codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264)
            .ok_or("H.264 encoder not found (FFmpeg may not be installed)")?;

        // Add video stream and get its index early
        let stream_index = {
            let mut video_stream = octx
                .add_stream(encoder_codec)
                .map_err(|e| format!("Failed to add video stream: {}", e))?;
            video_stream.index()
        };

        // Create encoder context with x264 options
        // We need to use lower-level FFmpeg API to pass codec-specific options
        let mut encoder = unsafe {
            use ffmpeg::ffi;
            use std::ptr;
            
            // Allocate codec context for the encoder
            let codec_ptr = encoder_codec.as_ptr();
            let mut codec_context = ffi::avcodec_alloc_context3(codec_ptr);
            if codec_context.is_null() {
                return Err("Failed to allocate codec context".to_string());
            }
            
            // Configure the codec context
            (*codec_context).codec_id = ffi::AVCodecID::AV_CODEC_ID_H264;
            (*codec_context).codec_type = ffi::AVMediaType::AVMEDIA_TYPE_VIDEO;
            (*codec_context).width = self.width as i32;
            (*codec_context).height = self.height as i32;
            (*codec_context).time_base = ffi::AVRational { num: 1, den: self.fps as i32 };
            (*codec_context).framerate = ffi::AVRational { num: self.fps as i32, den: 1 };
            (*codec_context).pix_fmt = ffi::AVPixelFormat::AV_PIX_FMT_YUV420P;
            (*codec_context).bit_rate = 5_000_000; // 5 Mbps
            
            // Create options dictionary for x264
            let mut opts: *mut ffi::AVDictionary = ptr::null_mut();
            let preset_key = std::ffi::CString::new("preset").unwrap();
            let preset_val = std::ffi::CString::new("medium").unwrap();
            
            ffi::av_dict_set(
                &mut opts,
                preset_key.as_ptr(),
                preset_val.as_ptr(),
                0,
            );
            
            // Open the codec with options
            let result = ffi::avcodec_open2(codec_context, codec_ptr, &mut opts);
            
            // Free the dictionary
            ffi::av_dict_free(&mut opts);
            
            if result < 0 {
                ffi::avcodec_free_context(&mut codec_context as *mut _);
                return Err(format!("Failed to open encoder with options: FFmpeg error {}", result));
            }
            
            // Wrap the codec context in ffmpeg-next's type system
            ffmpeg::codec::Context::wrap(codec_context, None)
                .encoder()
                .video()
                .map_err(|e| {
                    ffi::avcodec_free_context(&mut codec_context as *mut _);
                    format!("Failed to wrap encoder: {}", e)
                })?
        };

        // Set stream parameters
        {
            let mut video_stream = octx.stream_mut(stream_index).unwrap();
            video_stream.set_parameters(&encoder);
        }

        // Write file header
        octx.write_header()
            .map_err(|e| format!("Failed to write MP4 header: {}", e))?;

        println!("✍️  Encoding frames...");

        // Get time base for rescaling
        let time_base = octx.stream(stream_index).unwrap().time_base();

        // Encode each frame
        for (i, frame_data) in raw_frames.iter().enumerate() {
            if i == 0 {
                println!("📊 Processing frame 0 - Data size: {} bytes, Expected: {} bytes",
                    frame_data.len(), (self.width * self.height * 3) as usize);
            }
            
            // Create RGB frame
            let mut rgb_frame = ffmpeg::util::frame::video::Video::new(
                ffmpeg::format::Pixel::RGB24,
                self.width,
                self.height,
            );
            
            if i == 0 {
                println!("📊 RGB frame stride: {}, line_size: {}", 
                    rgb_frame.stride(0), self.width * 3);
            }
            
            // Copy RGB data line by line, respecting stride
            let stride = rgb_frame.stride(0);
            let line_size = self.width as usize * 3; // RGB24 = 3 bytes per pixel
            let rgb_data = rgb_frame.data_mut(0);
            
            for y in 0..self.height as usize {
                let src_offset = y * line_size;
                let dst_offset = y * stride;
                let src_line = &frame_data[src_offset..src_offset + line_size];
                let dst_line = &mut rgb_data[dst_offset..dst_offset + line_size];
                dst_line.copy_from_slice(src_line);
            }

            // Create YUV frame for encoder
            let mut yuv_frame = ffmpeg::util::frame::video::Video::new(
                ffmpeg::format::Pixel::YUV420P,
                self.width,
                self.height,
            );
            yuv_frame.set_pts(Some(i as i64));

            // Convert RGB to YUV using FFmpeg's software scaler
            let mut scaler = ffmpeg::software::scaling::Context::get(
                ffmpeg::format::Pixel::RGB24,
                self.width,
                self.height,
                ffmpeg::format::Pixel::YUV420P,
                self.width,
                self.height,
                ffmpeg::software::scaling::Flags::BILINEAR,
            )
            .map_err(|e| format!("Failed to create scaler: {}", e))?;

            scaler
                .run(&rgb_frame, &mut yuv_frame)
                .map_err(|e| format!("Failed to convert RGB to YUV: {}", e))?;

            // Send YUV frame to encoder
            encoder
                .send_frame(&yuv_frame)
                .map_err(|e| format!("Failed to encode frame {}: {}", i, e))?;

            // Receive encoded packets
            let mut packet = ffmpeg::codec::packet::Packet::empty();
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(stream_index);
                packet.rescale_ts(
                    ffmpeg::util::rational::Rational::new(1, self.fps as i32),
                    time_base,
                );
                packet.write_interleaved(&mut octx)
                    .map_err(|e| format!("Failed to write packet: {}", e))?;
            }

            // Progress indicator every 30 frames
            if (i + 1) % 30 == 0 {
                println!("  📊 Encoded {}/{} frames", i + 1, raw_frames.len());
            }
        }

        println!("🏁 Flushing encoder...");

        // Flush encoder (send EOF)
        encoder
            .send_eof()
            .map_err(|e| format!("Failed to send EOF: {}", e))?;

        // Receive remaining packets
        let mut packet = ffmpeg::codec::packet::Packet::empty();
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(stream_index);
            packet.write_interleaved(&mut octx)
                .map_err(|e| format!("Failed to write final packet: {}", e))?;
        }

        // Write file trailer
        octx.write_trailer()
            .map_err(|e| format!("Failed to write MP4 trailer: {}", e))?;

        println!("✅ Video encoded successfully!");

        Ok(())
    }

    /// Calculate video duration in seconds
    pub fn calculate_duration(&self, frame_count: usize) -> f64 {
        frame_count as f64 / self.fps as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_calculation() {
        let encoder = VideoEncoder::new(1920, 1080, 30);
        
        // 30 frames @ 30 fps = 1 second
        assert_eq!(encoder.calculate_duration(30), 1.0);
        
        // 300 frames @ 30 fps = 10 seconds
        assert_eq!(encoder.calculate_duration(300), 10.0);
        
        // 90 frames @ 30 fps = 3 seconds
        assert_eq!(encoder.calculate_duration(90), 3.0);
    }
}
