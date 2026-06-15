use ytdlp_core::{AudioFormat, ContentType, DownloadPreset, VideoContainer, VideoQuality};
use std::path::Path;

/// Translates a `DownloadPreset` and URL into the argument vector passed to yt-dlp.
pub fn build_ytdlp_args(url: &str, preset: &DownloadPreset, output_dir: &Path) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Always use newline-separated progress output for easy parsing
    args.push("--newline".into());
    args.push("--progress".into());
    args.push("--no-colors".into());

    // Output template
    let template = format!(
        "{}/{}",
        output_dir.display(),
        preset.output_template
    );
    args.extend(["--output".into(), template]);

    match preset.content_type {
        ContentType::VideoAudio => {
            let format_str = video_format_string(preset);
            args.extend(["-f".into(), format_str]);

            let ext = match preset.video_container {
                VideoContainer::Mp4 => "mp4",
                VideoContainer::Mkv => "mkv",
                VideoContainer::WebM => "webm",
                VideoContainer::Best => "mp4",
            };
            args.extend(["--merge-output-format".into(), ext.into()]);

            // Build format sort list
            let mut sort_parts = Vec::new();
            if preset.prefer_hdr {
                sort_parts.push("hdr:1");
            }
            if ext == "mp4" {
                sort_parts.push("vcodec:h264");
                sort_parts.push("acodec:aac");
            }
            
            if !sort_parts.is_empty() {
                args.extend(["--format-sort".into(), sort_parts.join(",")]);
            }
        }
        ContentType::AudioOnly => {
            args.push("--extract-audio".into());
            let fmt = match preset.audio_format {
                AudioFormat::Mp3 => "mp3",
                AudioFormat::Aac => "aac",
                AudioFormat::Opus => "opus",
                AudioFormat::Flac => "flac",
                AudioFormat::Best => "best",
            };
            args.extend(["--audio-format".into(), fmt.into()]);
            args.extend(["--audio-quality".into(), "0".into()]);
        }
        ContentType::SubtitlesOnly => {
            args.push("--skip-download".into());
            args.push("--write-subs".into());
            args.push("--write-auto-subs".into());
            let langs = preset.subtitles.languages.join(",");
            args.extend(["--sub-langs".into(), langs]);
        }
    }

    // Metadata and thumbnail embedding
    if preset.embed_metadata {
        args.push("--embed-metadata".into());
    }
    if preset.embed_thumbnail {
        args.push("--embed-thumbnail".into());
    }

    // Subtitle options (for video downloads)
    if matches!(preset.content_type, ContentType::VideoAudio)
        && preset.subtitles.download
    {
        args.push("--write-subs".into());
        if preset.subtitles.auto_generated {
            args.push("--write-auto-subs".into());
        }
        if preset.subtitles.embed {
            args.push("--embed-subs".into());
        }
        let langs = preset.subtitles.languages.join(",");
        args.extend(["--sub-langs".into(), langs]);
    }

    // SponsorBlock
    if preset.sponsorblock.enabled && !preset.sponsorblock.categories.is_empty() {
        let cats = preset.sponsorblock.categories.join(",");
        args.extend(["--sponsorblock-remove".into(), cats]);
    }

    // URL last
    args.push(url.to_string());

    args
}

fn video_format_string(preset: &DownloadPreset) -> String {
    let height = match preset.video_quality {
        VideoQuality::Best => return "bestvideo+bestaudio/best".into(),
        VideoQuality::Res8K => 4320,
        VideoQuality::Res4K => 2160,
        VideoQuality::Res1440p => 1440,
        VideoQuality::Res1080p => 1080,
        VideoQuality::Res720p => 720,
        VideoQuality::Res480p => 480,
        VideoQuality::Res360p => 360,
        VideoQuality::Res240p => 240,
        VideoQuality::Res144p => 144,
    };
    format!(
        "bestvideo[height<={height}]+bestaudio/best[height<={height}]",
        height = height
    )
}
