use serde::{Deserialize, Serialize};

// ── Download options ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    VideoAudio,
    AudioOnly,
    SubtitlesOnly,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::VideoAudio => write!(f, "Video + Audio"),
            ContentType::AudioOnly => write!(f, "Audio Only"),
            ContentType::SubtitlesOnly => write!(f, "Subtitles Only"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoContainer {
    Mp4,
    Mkv,
    WebM,
    Best,
}

impl VideoContainer {
    pub fn all_variants() -> Vec<Self> {
        vec![
            VideoContainer::Best,
            VideoContainer::Mp4,
            VideoContainer::Mkv,
            VideoContainer::WebM,
        ]
    }

    pub fn supports_hdr(&self) -> bool {
        match self {
            VideoContainer::Mp4 | VideoContainer::Mkv | VideoContainer::WebM | VideoContainer::Best => true,
        }
    }
}

impl std::fmt::Display for VideoContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoContainer::Mp4 => write!(f, "MP4"),
            VideoContainer::Mkv => write!(f, "MKV"),
            VideoContainer::WebM => write!(f, "WebM"),
            VideoContainer::Best => write!(f, "Best (auto)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Aac,
    Opus,
    Flac,
    Best,
}

impl AudioFormat {
    pub fn all_variants() -> Vec<Self> {
        vec![
            AudioFormat::Best,
            AudioFormat::Mp3,
            AudioFormat::Aac,
            AudioFormat::Opus,
            AudioFormat::Flac,
        ]
    }
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Mp3 => write!(f, "MP3"),
            AudioFormat::Aac => write!(f, "AAC"),
            AudioFormat::Opus => write!(f, "Opus"),
            AudioFormat::Flac => write!(f, "FLAC"),
            AudioFormat::Best => write!(f, "Best (auto)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoQuality {
    Best,
    Res8K,
    Res4K,
    Res1440p,
    Res1080p,
    Res720p,
    Res480p,
    Res360p,
    Res240p,
    Res144p,
}

impl VideoQuality {
    pub fn all_variants() -> Vec<Self> {
        vec![
            VideoQuality::Best,
            VideoQuality::Res8K,
            VideoQuality::Res4K,
            VideoQuality::Res1440p,
            VideoQuality::Res1080p,
            VideoQuality::Res720p,
            VideoQuality::Res480p,
            VideoQuality::Res360p,
            VideoQuality::Res240p,
            VideoQuality::Res144p,
        ]
    }
}

impl std::fmt::Display for VideoQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoQuality::Best => write!(f, "Best"),
            VideoQuality::Res8K => write!(f, "8K (4320p)"),
            VideoQuality::Res4K => write!(f, "4K (2160p)"),
            VideoQuality::Res1440p => write!(f, "1440p"),
            VideoQuality::Res1080p => write!(f, "1080p"),
            VideoQuality::Res720p => write!(f, "720p"),
            VideoQuality::Res480p => write!(f, "480p"),
            VideoQuality::Res360p => write!(f, "360p"),
            VideoQuality::Res240p => write!(f, "240p"),
            VideoQuality::Res144p => write!(f, "144p"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleOptions {
    pub download: bool,
    pub embed: bool,
    pub languages: Vec<String>,
    pub auto_generated: bool,
}

impl Default for SubtitleOptions {
    fn default() -> Self {
        Self {
            download: false,
            embed: false,
            languages: vec!["en".to_string()],
            auto_generated: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorBlockOptions {
    pub enabled: bool,
    pub categories: Vec<String>,
}

impl Default for SponsorBlockOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            categories: vec!["sponsor".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPreset {
    pub name: String,
    pub content_type: ContentType,
    pub video_quality: VideoQuality,
    pub video_container: VideoContainer,
    pub audio_format: AudioFormat,
    pub subtitles: SubtitleOptions,
    pub output_template: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub sponsorblock: SponsorBlockOptions,
    pub prefer_hdr: bool,
    pub use_gpu: bool,
}

impl Default for DownloadPreset {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            content_type: ContentType::VideoAudio,
            video_quality: VideoQuality::Best,
            video_container: VideoContainer::Best,
            audio_format: AudioFormat::Best,
            subtitles: SubtitleOptions::default(),
            output_template: "%(title)s.%(ext)s".to_string(),
            embed_thumbnail: false,
            embed_metadata: true,
            sponsorblock: SponsorBlockOptions::default(),
            prefer_hdr: true,
            use_gpu: true,
        }
    }
}

// ── Transcode options ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerFormat {
    Mp4,
    Mkv,
    WebM,
    Mov,
    Avi,
    Flv,
    Mp3,
    Aac,
    Flac,
    Ogg,
    Wav,
    Wma,
    M4a,
    ThreeGp,
    Ts,
    Vob,
    Gif,
}

impl ContainerFormat {
    pub fn all_variants() -> Vec<Self> {
        vec![
            ContainerFormat::Mp4,
            ContainerFormat::Mkv,
            ContainerFormat::WebM,
            ContainerFormat::Mov,
            ContainerFormat::Avi,
            ContainerFormat::Flv,
            ContainerFormat::Mp3,
            ContainerFormat::Aac,
            ContainerFormat::Flac,
            ContainerFormat::Ogg,
            ContainerFormat::Wav,
            ContainerFormat::Wma,
            ContainerFormat::M4a,
            ContainerFormat::ThreeGp,
            ContainerFormat::Ts,
            ContainerFormat::Vob,
            ContainerFormat::Gif,
        ]
    }

    pub fn supports_hdr(&self) -> bool {
        match self {
            ContainerFormat::Mp4
            | ContainerFormat::Mkv
            | ContainerFormat::WebM
            | ContainerFormat::Mov
            | ContainerFormat::Ts
            | ContainerFormat::Vob => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for ContainerFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerFormat::Mp4 => write!(f, "MP4"),
            ContainerFormat::Mkv => write!(f, "MKV"),
            ContainerFormat::WebM => write!(f, "WebM"),
            ContainerFormat::Mov => write!(f, "MOV"),
            ContainerFormat::Avi => write!(f, "AVI"),
            ContainerFormat::Flv => write!(f, "FLV"),
            ContainerFormat::Mp3 => write!(f, "MP3"),
            ContainerFormat::Aac => write!(f, "AAC"),
            ContainerFormat::Flac => write!(f, "FLAC"),
            ContainerFormat::Ogg => write!(f, "OGG"),
            ContainerFormat::Wav => write!(f, "WAV"),
            ContainerFormat::Wma => write!(f, "WMA"),
            ContainerFormat::M4a => write!(f, "M4A"),
            ContainerFormat::ThreeGp => write!(f, "3GP"),
            ContainerFormat::Ts => write!(f, "TS"),
            ContainerFormat::Vob => write!(f, "VOB"),
            ContainerFormat::Gif => write!(f, "GIF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    Av1,
    Vp9,
    Vp8,
    ProRes,
    Mpeg4,
    Mpeg2,
    Mpeg1,
    Theora,
    Mjpeg,
    Copy,
}

impl VideoCodec {
    pub fn all_variants() -> Vec<Self> {
        vec![
            VideoCodec::Copy,
            VideoCodec::H264,
            VideoCodec::H265,
            VideoCodec::Av1,
            VideoCodec::Vp9,
            VideoCodec::Vp8,
            VideoCodec::ProRes,
            VideoCodec::Mpeg4,
            VideoCodec::Mpeg2,
            VideoCodec::Mpeg1,
            VideoCodec::Theora,
            VideoCodec::Mjpeg,
        ]
    }
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoCodec::H264 => write!(f, "H.264 (AVC)"),
            VideoCodec::H265 => write!(f, "H.265 (HEVC)"),
            VideoCodec::Av1 => write!(f, "AV1"),
            VideoCodec::Vp9 => write!(f, "VP9"),
            VideoCodec::Vp8 => write!(f, "VP8"),
            VideoCodec::ProRes => write!(f, "ProRes"),
            VideoCodec::Mpeg4 => write!(f, "MPEG-4"),
            VideoCodec::Mpeg2 => write!(f, "MPEG-2"),
            VideoCodec::Mpeg1 => write!(f, "MPEG-1"),
            VideoCodec::Theora => write!(f, "Theora"),
            VideoCodec::Mjpeg => write!(f, "MJPEG"),
            VideoCodec::Copy => write!(f, "Copy (no re-encode)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    Source,
    Res8K,
    Res4K,
    Res1440p,
    Res1080p,
    Res720p,
    Res480p,
    Res360p,
    Res240p,
}

impl Resolution {
    pub fn all_variants() -> Vec<Self> {
        vec![
            Resolution::Source,
            Resolution::Res8K,
            Resolution::Res4K,
            Resolution::Res1440p,
            Resolution::Res1080p,
            Resolution::Res720p,
            Resolution::Res480p,
            Resolution::Res360p,
            Resolution::Res240p,
        ]
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resolution::Source => write!(f, "Source"),
            Resolution::Res8K => write!(f, "8K (4320p)"),
            Resolution::Res4K => write!(f, "4K (2160p)"),
            Resolution::Res1440p => write!(f, "1440p"),
            Resolution::Res1080p => write!(f, "1080p"),
            Resolution::Res720p => write!(f, "720p"),
            Resolution::Res480p => write!(f, "480p"),
            Resolution::Res360p => write!(f, "360p"),
            Resolution::Res240p => write!(f, "240p"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HdrPolicy {
    Preserve,
    ToneMapToSdr,
}

impl std::fmt::Display for HdrPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HdrPolicy::Preserve => write!(f, "Preserve HDR"),
            HdrPolicy::ToneMapToSdr => write!(f, "Tone-map to SDR"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Mp3,
    Opus,
    Flac,
    Vorbis,
    Copy,
}

impl AudioCodec {
    pub fn all_variants() -> Vec<Self> {
        vec![
            AudioCodec::Copy,
            AudioCodec::Aac,
            AudioCodec::Mp3,
            AudioCodec::Opus,
            AudioCodec::Flac,
            AudioCodec::Vorbis,
        ]
    }
}

impl std::fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioCodec::Aac => write!(f, "AAC"),
            AudioCodec::Mp3 => write!(f, "MP3"),
            AudioCodec::Opus => write!(f, "Opus"),
            AudioCodec::Flac => write!(f, "FLAC"),
            AudioCodec::Vorbis => write!(f, "Vorbis"),
            AudioCodec::Copy => write!(f, "Copy (no re-encode)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodingPreset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

impl EncodingPreset {
    pub fn all_variants() -> Vec<Self> {
        vec![
            EncodingPreset::Ultrafast,
            EncodingPreset::Superfast,
            EncodingPreset::Veryfast,
            EncodingPreset::Faster,
            EncodingPreset::Fast,
            EncodingPreset::Medium,
            EncodingPreset::Slow,
            EncodingPreset::Slower,
            EncodingPreset::Veryslow,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EncodingPreset::Ultrafast => "ultrafast",
            EncodingPreset::Superfast => "superfast",
            EncodingPreset::Veryfast => "veryfast",
            EncodingPreset::Faster => "faster",
            EncodingPreset::Fast => "fast",
            EncodingPreset::Medium => "medium",
            EncodingPreset::Slow => "slow",
            EncodingPreset::Slower => "slower",
            EncodingPreset::Veryslow => "veryslow",
        }
    }
}

impl std::fmt::Display for EncodingPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingPreset::Ultrafast => write!(f, "Ultrafast"),
            EncodingPreset::Superfast => write!(f, "Superfast"),
            EncodingPreset::Veryfast => write!(f, "Veryfast"),
            EncodingPreset::Faster => write!(f, "Faster"),
            EncodingPreset::Fast => write!(f, "Fast"),
            EncodingPreset::Medium => write!(f, "Medium"),
            EncodingPreset::Slow => write!(f, "Slow"),
            EncodingPreset::Slower => write!(f, "Slower"),
            EncodingPreset::Veryslow => write!(f, "Veryslow"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscodePreset {
    pub name: String,
    pub container: ContainerFormat,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub resolution: Resolution,
    pub hdr_policy: HdrPolicy,
    pub crf: u32,
    pub speed: EncodingPreset,
    pub audio_bitrate_kbps: Option<u32>,
    pub use_gpu: bool,
}

impl std::fmt::Display for TranscodePreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl TranscodePreset {
    pub fn builtin_presets() -> Vec<Self> {
        vec![
            TranscodePreset {
                name: "MP4 H.264 1080p SDR (Universal)".to_string(),
                container: ContainerFormat::Mp4,
                video_codec: VideoCodec::H264,
                audio_codec: AudioCodec::Aac,
                resolution: Resolution::Res1080p,
                hdr_policy: HdrPolicy::ToneMapToSdr,
                crf: 18,
                speed: EncodingPreset::Slow,
                audio_bitrate_kbps: Some(192),
                use_gpu: true,
            },
            TranscodePreset {
                name: "MOV ProRes 422 (Editing)".to_string(),
                container: ContainerFormat::Mov,
                video_codec: VideoCodec::ProRes,
                audio_codec: AudioCodec::Copy,
                resolution: Resolution::Source,
                hdr_policy: HdrPolicy::Preserve,
                crf: 0,
                speed: EncodingPreset::Medium,
                audio_bitrate_kbps: None,
                use_gpu: true,
            },
            TranscodePreset {
                name: "MKV H.265 4K HDR (Archive)".to_string(),
                container: ContainerFormat::Mkv,
                video_codec: VideoCodec::H265,
                audio_codec: AudioCodec::Aac,
                resolution: Resolution::Res4K,
                hdr_policy: HdrPolicy::Preserve,
                crf: 20,
                speed: EncodingPreset::Slow,
                audio_bitrate_kbps: None,
                use_gpu: true,
            },
            TranscodePreset {
                name: "WebM VP9 1440p HDR (Web)".to_string(),
                container: ContainerFormat::WebM,
                video_codec: VideoCodec::Vp9,
                audio_codec: AudioCodec::Opus,
                resolution: Resolution::Res1440p,
                hdr_policy: HdrPolicy::Preserve,
                crf: 30,
                speed: EncodingPreset::Medium,
                audio_bitrate_kbps: Some(192),
                use_gpu: true,
            },
            TranscodePreset {
                name: "GIF (Social Media)".to_string(),
                container: ContainerFormat::Gif,
                video_codec: VideoCodec::Mjpeg,
                audio_codec: AudioCodec::Copy,
                resolution: Resolution::Res480p,
                hdr_policy: HdrPolicy::ToneMapToSdr,
                crf: 4,
                speed: EncodingPreset::Medium,
                audio_bitrate_kbps: None,
                use_gpu: true,
            },
            TranscodePreset {
                name: "AVI Xvid 720p (Legacy)".to_string(),
                container: ContainerFormat::Avi,
                video_codec: VideoCodec::Mpeg4,
                audio_codec: AudioCodec::Mp3,
                resolution: Resolution::Res720p,
                hdr_policy: HdrPolicy::ToneMapToSdr,
                crf: 4,
                speed: EncodingPreset::Medium,
                audio_bitrate_kbps: Some(128),
                use_gpu: true,
            },
            TranscodePreset {
                name: "MP3 320kbps (Audio)".to_string(),
                container: ContainerFormat::Mp3,
                video_codec: VideoCodec::Copy,
                audio_codec: AudioCodec::Mp3,
                resolution: Resolution::Source,
                hdr_policy: HdrPolicy::ToneMapToSdr,
                crf: 0,
                speed: EncodingPreset::Medium,
                audio_bitrate_kbps: Some(320),
                use_gpu: true,
            },
            TranscodePreset {
                name: "FLAC (Lossless Audio)".to_string(),
                container: ContainerFormat::Flac,
                video_codec: VideoCodec::Copy,
                audio_codec: AudioCodec::Flac,
                resolution: Resolution::Source,
                hdr_policy: HdrPolicy::ToneMapToSdr,
                crf: 0,
                speed: EncodingPreset::Medium,
                audio_bitrate_kbps: None,
                use_gpu: true,
            },
        ]
    }
}
