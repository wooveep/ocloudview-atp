//! SPICE Display 通道
//!
//! 实现显示通道，用于接收视频流和图像数据。

use crate::{ProtocolError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use super::channel::{ChannelConnection, ChannelType};
use super::constants::*;
use super::messages::*;
use super::types::*;

/// Display 通道配置
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    /// pixmap 缓存大小
    pub pixmap_cache_size: u64,
    /// GLZ 字典窗口大小
    pub glz_dictionary_window_size: u32,
    /// 首选压缩类型
    pub preferred_compression: u8,
    /// 首选视频编解码器
    pub preferred_video_codec: u8,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            pixmap_cache_size: 32 * 1024 * 1024,         // 32MB
            glz_dictionary_window_size: 8 * 1024 * 1024, // 8MB
            preferred_compression: image_compression::AUTO_GLZ,
            preferred_video_codec: video_codec::VP8,
        }
    }
}

/// Surface 信息
#[derive(Debug, Clone)]
pub struct Surface {
    /// Surface ID
    pub id: u32,
    /// 宽度
    pub width: u32,
    /// 高度
    pub height: u32,
    /// 格式
    pub format: u32,
    /// 标志
    pub flags: u32,
    /// 是否为主 surface
    pub is_primary: bool,
}

/// 视频流信息
#[derive(Debug, Clone)]
pub struct VideoStream {
    /// 流 ID
    pub id: u32,
    /// Surface ID
    pub surface_id: u32,
    /// 编解码器类型
    pub codec_type: u8,
    /// 宽度
    pub width: u32,
    /// 高度
    pub height: u32,
    /// 标志
    pub flags: u8,
    /// 是否活跃
    pub active: bool,
}

/// Display 事件
#[derive(Debug, Clone)]
pub enum DisplayEvent {
    /// Surface 创建
    SurfaceCreated(Surface),
    /// Surface 销毁
    SurfaceDestroyed(u32),
    /// 显示模式变更
    ModeChanged { width: u32, height: u32, depth: u32 },
    /// 视频流创建
    StreamCreated(VideoStream),
    /// 视频流数据
    StreamData { stream_id: u32, data: Vec<u8> },
    /// 视频流销毁
    StreamDestroyed(u32),
    /// 显示器配置更新
    MonitorsConfig(Vec<MonitorConfig>),
    /// 绘图命令（简化表示）
    DrawCommand {
        surface_id: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
}

/// Display 事件处理器
pub trait DisplayEventHandler: Send + Sync {
    fn on_event(&self, event: DisplayEvent);
}

/// SPICE Display 通道
///
/// 用于接收虚拟机的显示输出
pub struct DisplayChannel {
    /// 底层通道连接
    connection: ChannelConnection,
    /// 配置
    config: DisplayConfig,
    /// Surface 列表
    surfaces: HashMap<u32, Surface>,
    /// 视频流列表
    streams: HashMap<u32, VideoStream>,
    /// 当前分辨率
    current_resolution: Option<Resolution>,
    /// 事件处理器
    event_handler: Option<Arc<dyn DisplayEventHandler>>,
    /// 接收的帧数
    frame_count: u64,
}

impl DisplayChannel {
    /// 创建新的 Display 通道
    pub fn new(channel_id: u8) -> Self {
        Self {
            connection: ChannelConnection::new(ChannelType::Display, channel_id),
            config: DisplayConfig::default(),
            surfaces: HashMap::new(),
            streams: HashMap::new(),
            current_resolution: None,
            event_handler: None,
            frame_count: 0,
        }
    }

    /// 使用配置创建
    pub fn with_config(channel_id: u8, config: DisplayConfig) -> Self {
        let mut channel = Self::new(channel_id);
        channel.config = config;
        channel
    }

    /// 设置事件处理器
    pub fn set_event_handler(&mut self, handler: Arc<dyn DisplayEventHandler>) {
        self.event_handler = Some(handler);
    }

    /// 连接到服务器
    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        connection_id: u32,
        password: Option<&str>,
    ) -> Result<()> {
        self.connection
            .connect(host, port, connection_id, password)
            .await?;

        // 发送 Display 初始化消息
        self.send_init().await?;

        debug!("Display 通道已连接");
        Ok(())
    }

    /// 发送初始化消息
    async fn send_init(&mut self) -> Result<()> {
        // TODO: 根据 SPICE 协议，发送 MSGC_DISPLAY_INIT
        // let init = MsgcDisplayInit::new();
        // self.connection.send_message(msgc_display::INIT, &init.to_bytes()).await?;

        debug!("Display 初始化消息已发送");
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<()> {
        self.surfaces.clear();
        self.streams.clear();
        self.connection.disconnect().await
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }

    /// 获取当前分辨率
    pub fn current_resolution(&self) -> Option<Resolution> {
        self.current_resolution
    }

    /// 获取 Surface 列表
    pub fn surfaces(&self) -> &HashMap<u32, Surface> {
        &self.surfaces
    }

    /// 获取主 Surface
    pub fn primary_surface(&self) -> Option<&Surface> {
        self.surfaces.values().find(|s| s.is_primary)
    }

    /// 获取视频流列表
    pub fn streams(&self) -> &HashMap<u32, VideoStream> {
        &self.streams
    }

    /// 获取帧计数
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// 处理服务器消息
    pub async fn process_events(&mut self) -> Result<Option<DisplayEvent>> {
        let (msg_type, data) = self.connection.receive_message().await?;

        let event = match msg_type {
            // SPICE_MSG_DISPLAY_MODE
            101 => {
                if let Some(mode) = MsgDisplayMode::from_bytes(&data) {
                    self.current_resolution = Some(Resolution::new(mode.x_res, mode.y_res));
                    debug!("显示模式: {}x{}x{}", mode.x_res, mode.y_res, mode.bits);
                    Some(DisplayEvent::ModeChanged {
                        width: mode.x_res,
                        height: mode.y_res,
                        depth: mode.bits,
                    })
                } else {
                    None
                }
            }
            // SPICE_MSG_DISPLAY_MARK
            102 => {
                debug!("收到显示标记");
                None
            }
            // SPICE_MSG_DISPLAY_RESET
            103 => {
                debug!("显示重置");
                self.surfaces.clear();
                self.streams.clear();
                None
            }
            // SPICE_MSG_DISPLAY_SURFACE_CREATE
            315 => {
                if let Some(create) = MsgDisplaySurfaceCreate::from_bytes(&data) {
                    let surface = Surface {
                        id: create.surface_id,
                        width: create.width,
                        height: create.height,
                        format: create.format,
                        flags: create.flags,
                        is_primary: (create.flags & 1) != 0,
                    };
                    debug!(
                        "Surface 创建: id={}, {}x{}",
                        surface.id, surface.width, surface.height
                    );
                    self.surfaces.insert(surface.id, surface.clone());
                    Some(DisplayEvent::SurfaceCreated(surface))
                } else {
                    None
                }
            }
            // SPICE_MSG_DISPLAY_SURFACE_DESTROY
            316 => {
                if data.len() >= 4 {
                    let surface_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    debug!("Surface 销毁: id={}", surface_id);
                    self.surfaces.remove(&surface_id);
                    Some(DisplayEvent::SurfaceDestroyed(surface_id))
                } else {
                    None
                }
            }
            // SPICE_MSG_DISPLAY_STREAM_CREATE
            122 => {
                // TODO: 解析完整的流创建消息
                //
                // 参考：spice-protocol/spice/protocol.h 中的 SpiceMsgDisplayStreamCreate
                //
                // 消息结构（按字节顺序）：
                // struct SpiceMsgDisplayStreamCreate {
                //     uint32_t surface_id;      // 偏移 0: 目标 surface ID
                //     uint32_t id;              // 偏移 4: 流 ID
                //     uint8_t flags;            // 偏移 8: 标志位
                //     uint8_t codec_type;       // 偏移 9: 编解码器类型
                //     uint64_t stamp;           // 偏移 10: 时间戳（未使用可忽略）
                //     uint32_t stream_width;    // 偏移 18: 流宽度
                //     uint32_t stream_height;   // 偏移 22: 流高度
                //     uint32_t src_width;       // 偏移 26: 源宽度
                //     uint32_t src_height;      // 偏移 30: 源高度
                //     SpiceRect dest;           // 偏移 34: 目标矩形 (16 字节: top, left, bottom, right)
                //     SpiceClip clip;           // 偏移 50: 裁剪信息
                // }
                //
                // 实现步骤：
                // ```rust
                // if data.len() >= 34 {
                //     let surface_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                //     let stream_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                //     let flags = data[8];
                //     let codec_type = data[9];
                //     // 跳过 stamp (8 字节)
                //     let width = u32::from_le_bytes([data[18], data[19], data[20], data[21]]);
                //     let height = u32::from_le_bytes([data[22], data[23], data[24], data[25]]);
                //
                //     let stream = VideoStream {
                //         id: stream_id,
                //         surface_id,
                //         codec_type,
                //         width,
                //         height,
                //         flags,
                //         active: true,
                //     };
                //
                //     debug!("视频流创建: id={}, codec={}, {}x{}", stream_id, codec_type, width, height);
                //     self.streams.insert(stream_id, stream.clone());
                //     Some(DisplayEvent::StreamCreated(stream))
                // } else {
                //     None
                // }
                // ```
                //
                // 编解码器类型（在 constants.rs 中定义）：
                //   - MJPEG = 1
                //   - VP8 = 2
                //   - H264 = 3
                //   - VP9 = 4
                //   - H265 = 5
                //
                debug!("视频流创建");
                self.frame_count += 1;
                None
            }
            // SPICE_MSG_DISPLAY_STREAM_DATA
            123 => {
                // TODO: 解析视频流数据并解码
                //
                // 参考：spice-protocol/spice/protocol.h 中的 SpiceMsgDisplayStreamData
                //
                // 消息结构：
                // struct SpiceMsgDisplayStreamData {
                //     uint32_t id;              // 偏移 0: 流 ID
                //     uint32_t multi_media_time; // 偏移 4: 时间戳
                //     uint32_t data_size;       // 偏移 8: 数据大小
                //     uint8_t data[];           // 偏移 12: 编码的视频帧数据
                // }
                //
                // 实现步骤：
                // 1. 提取流 ID 和帧数据
                //    ```rust
                //    if data.len() >= 12 {
                //        let stream_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                //        let mm_time = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                //        let data_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
                //        let frame_data = &data[12..];
                //
                //        trace!("视频流数据: stream_id={}, size={}, time={}", stream_id, data_size, mm_time);
                //    ```
                //
                // 2. 根据流的编解码器类型解码（需要对应的解码库）
                //    - VP8/VP9: 使用 vpx-rs 或 libvpx-sys crate
                //      ```rust
                //      // 伪代码示例
                //      if let Some(stream) = self.streams.get(&stream_id) {
                //          match stream.codec_type {
                //              video_codec::VP8 => {
                //                  // TODO: 使用 vpx decoder 解码
                //                  // let decoder = vpx::Decoder::new(vpx::VideoCodecId::VP8)?;
                //                  // let decoded_frame = decoder.decode(frame_data)?;
                //              }
                //              video_codec::MJPEG => {
                //                  // TODO: 使用 image crate 解码 JPEG
                //                  // let img = image::load_from_memory_with_format(frame_data, ImageFormat::Jpeg)?;
                //              }
                //              video_codec::H264 => {
                //                  // TODO: 使用 openh264 或 ffmpeg 解码
                //              }
                //              _ => {
                //                  warn!("不支持的编解码器: {}", stream.codec_type);
                //              }
                //          }
                //      }
                //      ```
                //
                // 3. 触发事件并更新帧计数
                //    ```rust
                //        self.frame_count += 1;
                //        Some(DisplayEvent::StreamData {
                //            stream_id,
                //            data: frame_data.to_vec(),
                //        })
                //    } else {
                //        None
                //    }
                //    ```
                //
                // 需要的依赖（根据支持的编解码器）：
                //   - image = "0.24" (MJPEG)
                //   - vpx-rs = "0.1" 或 libvpx-sys (VP8/VP9)
                //   - openh264 = "0.4" (H264)
                //
                // 注意：对于负载测试场景，可能不需要实际解码，只需要：
                //   1. 统计接收到的帧数和数据量
                //   2. 验证数据完整性
                //   3. 测量帧率和带宽
                //
                trace!("视频流数据: {} bytes", data.len());
                self.frame_count += 1;
                None
            }
            // SPICE_MSG_DISPLAY_STREAM_DESTROY
            125 => {
                if data.len() >= 4 {
                    let stream_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    debug!("视频流销毁: id={}", stream_id);
                    self.streams.remove(&stream_id);
                    Some(DisplayEvent::StreamDestroyed(stream_id))
                } else {
                    None
                }
            }
            // SPICE_MSG_DISPLAY_STREAM_DESTROY_ALL
            126 => {
                debug!("所有视频流销毁");
                self.streams.clear();
                None
            }
            // SPICE_MSG_DISPLAY_MONITORS_CONFIG
            317 => {
                if let Some(config) = MsgDisplayMonitorsConfig::from_bytes(&data) {
                    debug!("显示器配置: {} 个显示器", config.count);
                    Some(DisplayEvent::MonitorsConfig(config.monitors))
                } else {
                    None
                }
            }
            // 绘图命令 (302-314)
            302..=314 => {
                // TODO: 解析和处理 SPICE 绘图命令
                //
                // 参考：spice-protocol/spice/protocol.h 中的绘图消息定义
                //
                // 主要绘图命令类型：
                //   302: SPICE_MSG_DISPLAY_DRAW_FILL       - 填充矩形
                //   303: SPICE_MSG_DISPLAY_DRAW_OPAQUE     - 不透明绘制
                //   304: SPICE_MSG_DISPLAY_DRAW_COPY       - 复制操作
                //   305: SPICE_MSG_DISPLAY_DRAW_BLEND      - 混合操作
                //   306: SPICE_MSG_DISPLAY_DRAW_BLACKNESS  - 填充黑色
                //   307: SPICE_MSG_DISPLAY_DRAW_WHITENESS  - 填充白色
                //   308: SPICE_MSG_DISPLAY_DRAW_INVERS     - 反色
                //   309: SPICE_MSG_DISPLAY_DRAW_ROP3       - 三元光栅操作
                //   310: SPICE_MSG_DISPLAY_DRAW_STROKE     - 描边
                //   311: SPICE_MSG_DISPLAY_DRAW_TEXT       - 文本绘制
                //   312: SPICE_MSG_DISPLAY_DRAW_TRANSPARENT - 透明绘制
                //   313: SPICE_MSG_DISPLAY_DRAW_ALPHA_BLEND - Alpha 混合
                //   314: SPICE_MSG_DISPLAY_DRAW_COMPOSITE  - 复合操作
                //
                // 通用绘图消息头部结构（SpiceMsgDisplayBase）：
                // struct SpiceMsgDisplayBase {
                //     uint32_t surface_id;      // 偏移 0: 目标 surface
                //     SpiceRect box;            // 偏移 4: 边界框 (16 字节)
                //     SpiceClip clip;           // 偏移 20: 裁剪信息
                // }
                //
                // SpiceRect 结构（16 字节）：
                //   int32_t top;
                //   int32_t left;
                //   int32_t bottom;
                //   int32_t right;
                //
                // 实现示例（以 DRAW_FILL 为例）：
                // ```rust
                // if msg_type == 302 { // DRAW_FILL
                //     if data.len() >= 20 {
                //         let surface_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                //         let top = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                //         let left = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
                //         let bottom = i32::from_le_bytes([data[12], data[13], data[14], data[15]]);
                //         let right = i32::from_le_bytes([data[16], data[17], data[18], data[19]]);
                //
                //         let width = (right - left) as u32;
                //         let height = (bottom - top) as u32;
                //
                //         trace!("绘图命令 FILL: surface={}, rect=({},{},{}x{})",
                //                surface_id, left, top, width, height);
                //
                //         self.frame_count += 1;
                //         return Ok(Some(DisplayEvent::DrawCommand {
                //             surface_id,
                //             x: left,
                //             y: top,
                //             width,
                //             height,
                //         }));
                //     }
                // }
                // ```
                //
                // 对于负载测试场景的简化实现：
                //   1. 只需解析基础头部获取 surface_id 和边界框
                //   2. 统计绘图命令的数量和类型分布
                //   3. 不需要实际执行绘图操作
                //   4. 可以记录绘制区域大小用于带宽计算
                //
                // 完整实现需要：
                //   - 解析各种图像格式（QUIC, LZ, GLZ 压缩）
                //   - 维护 pixmap 缓存
                //   - 实现光栅操作（ROP）
                //   - 处理调色板和颜色转换
                //
                // 参考资料：
                //   - spice-gtk/src/channel-display.c: display_handle_*() 函数
                //   - spice-gtk/src/spice-glib-enums.h: 绘图类型枚举
                //
                trace!("绘图命令: type={}", msg_type);
                self.frame_count += 1;
                None
            }
            _ => {
                trace!("Display 通道消息: type={}, size={}", msg_type, data.len());
                None
            }
        };

        // 触发事件处理器
        if let (Some(evt), Some(handler)) = (&event, &self.event_handler) {
            handler.on_event(evt.clone());
        }

        Ok(event)
    }

    /// 设置首选压缩类型
    pub async fn set_preferred_compression(&mut self, compression: u8) -> Result<()> {
        self.config.preferred_compression = compression;

        // TODO: 发送 MSGC_DISPLAY_PREFERRED_COMPRESSION
        // self.connection.send_message(msgc_display::PREFERRED_COMPRESSION, &[compression]).await?;

        debug!("设置首选压缩: {}", compression);
        Ok(())
    }

    /// 设置首选视频编解码器
    pub async fn set_preferred_video_codec(&mut self, codec: u8) -> Result<()> {
        self.config.preferred_video_codec = codec;

        // TODO: 发送 MSGC_DISPLAY_PREFERRED_VIDEO_CODEC_TYPE
        // self.connection.send_message(msgc_display::PREFERRED_VIDEO_CODEC_TYPE, &[codec]).await?;

        debug!("设置首选视频编解码器: {}", codec);
        Ok(())
    }
}

/// 简单的帧计数事件处理器
pub struct FrameCounterHandler {
    count: Arc<RwLock<u64>>,
}

impl FrameCounterHandler {
    pub fn new() -> Self {
        Self {
            count: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get_count(&self) -> u64 {
        *self.count.read().await
    }
}

impl Default for FrameCounterHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayEventHandler for FrameCounterHandler {
    fn on_event(&self, event: DisplayEvent) {
        match event {
            DisplayEvent::StreamData { .. } | DisplayEvent::DrawCommand { .. } => {
                let count = self.count.clone();
                tokio::spawn(async move {
                    let mut c = count.write().await;
                    *c += 1;
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_config_default() {
        let config = DisplayConfig::default();
        assert_eq!(config.pixmap_cache_size, 32 * 1024 * 1024);
        assert_eq!(config.preferred_compression, image_compression::AUTO_GLZ);
    }

    #[test]
    fn test_surface() {
        let surface = Surface {
            id: 1,
            width: 1920,
            height: 1080,
            format: 32,
            flags: 1,
            is_primary: true,
        };
        assert!(surface.is_primary);
    }
}
