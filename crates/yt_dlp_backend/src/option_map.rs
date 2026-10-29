use std::path::Path;
use ytdlp_core::{AudioFormat, ContentType, DownloadPreset, VideoContainer, VideoQuality};

/// Translates a `DownloadPreset` and URL into the argument vector passed to yt-dlp.
pub fn build_ytdlp_args(url: &str, preset: &DownloadPreset, output_dir: &Path) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Always use newline-separated progress output for easy parsing
    args.push("--newline".into());
    args.push("--progress".into());
    args.push("--no-colors".into());

    // Output template
    let template = format!("{}/{}", output_dir.display(), preset.output_template);
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
    if matches!(preset.content_type, ContentType::VideoAudio) && preset.subtitles.download {
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

    // Delimiter to prevent option injection from URLs starting with '-'
    args.push("--".into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use ytdlp_core::SubtitleOptions;

    #[test]
    fn test_build_ytdlp_args_default_video() {
        let preset = DownloadPreset::default();
        let out_dir = PathBuf::from("/downloads");
        let args = build_ytdlp_args("https://example.com/watch?v=123", &preset, &out_dir);

        assert!(args.contains(&"--newline".to_string()));
        assert!(args.contains(&"--progress".to_string()));
        assert!(args.contains(&"--no-colors".to_string()));
        assert!(args.contains(&"-f".to_string()));
        assert_eq!(args.last().unwrap(), "https://example.com/watch?v=123");
        assert_eq!(args[args.len() - 2], "--");

        // Merged output format
        let merge_idx = args
            .iter()
            .position(|a| a == "--merge-output-format")
            .unwrap();
        assert_eq!(args[merge_idx + 1], "mp4");
    }

    #[test]
    fn test_build_ytdlp_args_audio_only() {
        let preset = DownloadPreset {
            content_type: ContentType::AudioOnly,
            audio_format: AudioFormat::Mp3,
            ..Default::default()
        };

        let out_dir = PathBuf::from("/music");
        let args = build_ytdlp_args("https://example.com/song", &preset, &out_dir);

        assert!(args.contains(&"--extract-audio".to_string()));
        let fmt_idx = args.iter().position(|a| a == "--audio-format").unwrap();
        assert_eq!(args[fmt_idx + 1], "mp3");
        let qual_idx = args.iter().position(|a| a == "--audio-quality").unwrap();
        assert_eq!(args[qual_idx + 1], "0");
        assert_eq!(args.last().unwrap(), "https://example.com/song");
    }

    #[test]
    fn test_build_ytdlp_args_subtitles_only() {
        let preset = DownloadPreset {
            content_type: ContentType::SubtitlesOnly,
            subtitles: SubtitleOptions {
                download: true,
                embed: false,
                auto_generated: true,
                languages: vec!["en".into(), "es".into()],
            },
            ..Default::default()
        };

        let out_dir = PathBuf::from("/subs");
        let args = build_ytdlp_args("https://example.com/video", &preset, &out_dir);

        assert!(args.contains(&"--skip-download".to_string()));
        assert!(args.contains(&"--write-subs".to_string()));
        assert!(args.contains(&"--write-auto-subs".to_string()));
        let langs_idx = args.iter().position(|a| a == "--sub-langs").unwrap();
        assert_eq!(args[langs_idx + 1], "en,es");
    }

    #[test]
    fn test_build_ytdlp_args_resolutions() {
        let out_dir = PathBuf::from("/vids");

        let preset1080 = DownloadPreset {
            video_quality: VideoQuality::Res1080p,
            ..Default::default()
        };
        let args1080 = build_ytdlp_args("https://example.com", &preset1080, &out_dir);
        let f_idx = args1080.iter().position(|a| a == "-f").unwrap();
        assert_eq!(
            args1080[f_idx + 1],
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]"
        );

        let preset4k = DownloadPreset {
            video_quality: VideoQuality::Res4K,
            ..Default::default()
        };
        let args4k = build_ytdlp_args("https://example.com", &preset4k, &out_dir);
        let f_idx4k = args4k.iter().position(|a| a == "-f").unwrap();
        assert_eq!(
            args4k[f_idx4k + 1],
            "bestvideo[height<=2160]+bestaudio/best[height<=2160]"
        );
    }

    #[test]
    fn test_build_ytdlp_args_metadata_and_hdr() {
        let preset = DownloadPreset {
            embed_metadata: true,
            embed_thumbnail: true,
            prefer_hdr: true,
            ..Default::default()
        };

        let out_dir = PathBuf::from("/vids");
        let args = build_ytdlp_args("https://example.com", &preset, &out_dir);

        assert!(args.contains(&"--embed-metadata".to_string()));
        assert!(args.contains(&"--embed-thumbnail".to_string()));
        let sort_idx = args.iter().position(|a| a == "--format-sort").unwrap();
        assert!(args[sort_idx + 1].contains("hdr:1"));
    }
}
