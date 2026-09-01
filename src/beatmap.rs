//! .osu beatmap parsing and the lazer processing pipeline
//! (difficulty, timing points, hit objects, stacking, nested object creation).

use crate::path::{PathControlPoint, PathType, SliderPath};
use crate::samples::SampleData;
use crate::vec2::{precision, Vec2};

pub const EARLY_VERSION_TIMING_OFFSET: f64 = 24.0;
const FIRST_LAZER_VERSION: i32 = 128;

#[derive(Clone, Copy, Debug)]
pub struct Difficulty {
    pub hp: f32,
    pub cs: f32,
    pub od: f32,
    pub ar: f32,
    pub slider_multiplier: f32,
    pub slider_tick_rate: f32,
}

impl Default for Difficulty {
    fn default() -> Self {
        Difficulty {
            hp: 5.0,
            cs: 5.0,
            od: 5.0,
            ar: 5.0,
            slider_multiplier: 1.4,
            slider_tick_rate: 1.0,
        }
    }
}

/// `IBeatmapDifficultyInfo.DifficultyRange(difficulty)` = (difficulty - 5) / 5.
fn difficulty_range_normalized(difficulty: f64) -> f64 {
    (difficulty - 5.0) / 5.0
}

/// `IBeatmapDifficultyInfo.DifficultyRange(difficulty, min, mid, max)`.
pub fn difficulty_range(difficulty: f64, min: f64, mid: f64, max: f64) -> f64 {
    if difficulty > 5.0 {
        mid + (max - mid) * difficulty_range_normalized(difficulty)
    } else if difficulty < 5.0 {
        mid + (mid - min) * difficulty_range_normalized(difficulty)
    } else {
        mid
    }
}

pub fn difficulty_range_int(difficulty: f64, min: f64, mid: f64, max: f64) -> i32 {
    difficulty_range(difficulty, min, mid, max) as i32
}

/// `LegacyRulesetExtensions.CalculateScaleFromCircleSize(cs, applyFudge: true)`.
pub fn calculate_scale_from_circle_size(cs: f32) -> f32 {
    const BROKEN_GAMEFIELD_ROUNDING_ALLOWANCE: f32 = 1.00041;
    (1.0f32 - 0.7f32 * difficulty_range_normalized(cs as f64) as f32) / 2.0
        * BROKEN_GAMEFIELD_ROUNDING_ALLOWANCE
}

#[derive(Clone, Copy, Debug)]
pub struct TimingPoint {
    pub time: f64,
    pub beat_length: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct DifficultyPoint {
    pub time: f64,
    pub slider_velocity: f64,
    pub generate_ticks: bool,
}

pub enum RawHitObject {
    Circle { pos: Vec2 },
    Slider { pos: Vec2, path: SliderPath, repeat_count: usize },
    Spinner { end_time: f64 },
}

pub struct ParsedObject {
    pub start_time: f64,
    pub new_combo: bool,
    pub raw: RawHitObject,
    /// Slider velocity multiplier from the difficulty point active at start time (decode time).
    pub slider_velocity_multiplier: f64,
    /// Colour-skip count from the hit object type bits 4-6 (`ComboOffset`).
    pub combo_offset: u32,
}

/// `[General]` fields that are not judgement-relevant (those - `Mode`,
/// `StackLeniency` - stay directly on [`Beatmap`]). Absent flags stay
/// `None` so consumers can apply their own defaults.
#[derive(Clone, Debug, Default)]
pub struct GeneralInfo {
    pub audio_filename: Option<String>,
    pub audio_lead_in: f64,
    /// `-1` (unspecified) is stored as `None`.
    pub preview_time: Option<f64>,
    /// Raw countdown setting (0 = none, 1 = normal, 2 = half, 3 = double).
    pub countdown: Option<f64>,
    /// Raw `[General] SampleSet` value ("Normal"/"Soft"/"Drum"); the
    /// resolved per-point banks live in [`crate::samples`].
    pub sample_set: Option<String>,
    pub sample_volume: Option<i32>,
    pub letterbox_in_breaks: Option<bool>,
    pub widescreen_storyboard: Option<bool>,
    pub skin_preference: Option<String>,
}

/// `[Metadata]` section.
#[derive(Clone, Debug, Default)]
pub struct BeatmapMetadata {
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    /// Difficulty name (e.g. "Insane").
    pub version: String,
    pub source: String,
    /// Whitespace-split tags.
    pub tags: Vec<String>,
    /// Online IDs; absent lines keep lazer's "unknown" -1.
    pub beatmap_id: i64,
    pub beatmap_set_id: i64,
}

/// `[Editor]` section (raw values; only useful for tooling).
#[derive(Clone, Debug, Default)]
pub struct EditorInfo {
    pub bookmarks: Vec<f64>,
    pub distance_spacing: f64,
    pub beat_divisor: i32,
    pub grid_size: i32,
    pub timeline_zoom: f64,
}

/// `[Events]` video line (`Video,offset,"file.mp4"` or the legacy
/// `1,offset,"file.mp4"`).
#[derive(Clone, Debug)]
pub struct VideoEvent {
    pub start_time: f64,
    pub path: String,
}

#[derive(Default)]
pub struct Beatmap {
    pub version: i32,
    pub stack_leniency: f32,
    pub difficulty: Difficulty,
    pub timing_points: Vec<TimingPoint>,
    pub difficulty_points: Vec<DifficultyPoint>,
    pub objects: Vec<ParsedObject>,
    /// Beatmap-defined combo colours from the `[Colours]` section (sRGB 0-255).
    pub combo_colours: Vec<[u8; 3]>,
    /// `[Events]` break periods (`2,start,end`; end clamped to >= start),
    /// in file order - `LegacyBeatmapDecoder`'s `LegacyEventType.Break`.
    /// The health processor skips draining across these.
    pub breaks: Vec<(f64, f64)>,
    /// `[General]` fields not needed for judgement (audio, preview,
    /// letterboxing, ...).
    pub general: GeneralInfo,
    /// `[Metadata]` (title/artist/creator/version/source/tags/online IDs).
    pub metadata: BeatmapMetadata,
    /// `[Editor]` bookmarks and spacing settings.
    pub editor: EditorInfo,
    /// `[Events]` background image filename (`0,0,"bg.jpg",...`), first
    /// non-empty line wins.
    pub background: Option<String>,
    /// `[Events]` video (`Video,offset,"file.mp4"` / legacy `1,offset,...`).
    pub video: Option<VideoEvent>,
    /// Sample-side data (per-timing-point banks/volumes, per-object
    /// hitSample overrides) for hitsound synthesis.
    pub sample_data: SampleData,
}

/// Rightmost binary search: last point with `time <= t`, else None.
fn point_at<T: Copy>(points: &[T], t: f64, time_of: impl Fn(&T) -> f64) -> Option<T> {
        let mut lo = 0usize;
        let mut hi = points.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if time_of(&points[mid]) <= t {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 {
            None
        } else {
            Some(points[lo - 1])
        }
}

impl Beatmap {
    pub fn timing_point_at(&self, t: f64) -> TimingPoint {
        // ControlPointInfo.TimingPointAt falls back to the FIRST timing point
        // (even if it starts after `t`), else the default (BeatLength 1000).
        point_at(&self.timing_points, t, |p| p.time).unwrap_or(match self.timing_points.first() {
            Some(first) => *first,
            None => TimingPoint { time: f64::NEG_INFINITY, beat_length: 1000.0 },
        })
    }

    pub fn difficulty_point_at(&self, t: f64) -> DifficultyPoint {
        point_at(&self.difficulty_points, t, |p| p.time).unwrap_or(DifficultyPoint {
            time: f64::NEG_INFINITY,
            slider_velocity: 1.0,
            generate_ticks: true,
        })
    }
}

fn parse_f32(s: &str) -> f32 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    trimmed.parse::<f32>().unwrap_or(0.0)
}

fn parse_f64(s: &str) -> f64 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    trimmed.parse::<f64>().unwrap_or(0.0)
}

fn parse_i32(s: &str) -> i32 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0;
    }
    trimmed.parse::<i32>().unwrap_or(0)
}

fn parse_i64(s: &str) -> i64 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0;
    }
    trimmed.parse::<i64>().unwrap_or(0)
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

/// Stable-format coordinate parsing: `(int)Parsing.ParseFloat(x)`.
fn parse_coord(s: &str, lazer_format: bool) -> f32 {
    if lazer_format {
        parse_f32(s).clamp(-131072.0, 131072.0)
    } else {
        parse_f32(s).clamp(-131072.0, 131072.0) as i32 as f32
    }
}

fn convert_path_type(s: &str) -> Option<PathType> {
    match s.chars().next()? {
        'C' => Some(PathType::CATMULL),
        'B' => {
            // "B" or "B<n>" (B-spline of degree n).
            if s.len() > 1 {
                if let Ok(degree) = s[1..].parse::<i32>() {
                    return Some(PathType::bspline(degree));
                }
            }
            Some(PathType::BEZIER)
        }
        'L' => Some(PathType::LINEAR),
        'P' => Some(PathType::PERFECT_CURVE),
        _ => None,
    }
}

struct PendingSegment {
    kind: PathType,
    start_index: usize,
}

/// Port of `ConvertHitObjectParser.convertPathString`: produces a flat
/// control point list. On implicit segment splits (duplicate control points)
/// the FIRST of the duplicated pair stays in the list as a typed boundary
/// marker and the second is dropped; `SliderPath` later splits at typed
/// points, sharing the boundary point between segments.
fn convert_path_string(point_string: &str, offset: Vec2, lazer_format: bool) -> Vec<PathControlPoint> {
    let split: Vec<&str> = point_string.split('|').collect();

    let mut points: Vec<Vec2> = Vec::with_capacity(split.len());
    let mut segments: Vec<PendingSegment> = Vec::with_capacity(split.len());

    for s in &split {
        if let Some(kind) = convert_path_type(s) {
            segments.push(PendingSegment { kind, start_index: points.len() });
            // First segment is prepended by an extra zero point.
            if points.is_empty() {
                points.push(Vec2::ZERO);
            }
        } else {
            let vertex_split: Vec<&str> = s.split(':').collect();
            let mut pos = Vec2::new(
                parse_coord(vertex_split[0], lazer_format),
                parse_coord(vertex_split[1], lazer_format),
            );
            pos = pos - offset;
            points.push(pos);
        }
    }

    let mut control_points: Vec<PathControlPoint> = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        let end = if i < segments.len() - 1 { segments[i + 1].start_index } else { points.len() };
        let pts = &points[segment.start_index..end];
        let end_point = if i < segments.len() - 1 { Some(points[segments[i + 1].start_index]) } else { None };
        convert_points(segment.kind, pts, end_point, &mut control_points, lazer_format);
    }

    control_points
}

/// Appends the flat control point list for one explicit segment, mirroring
/// `ConvertHitObjectParser.convertPoints` (implicit splits on duplicate
/// points, perfect-curve edge rules).
fn convert_points(
    mut kind: PathType,
    points: &[Vec2],
    end_point: Option<Vec2>,
    output: &mut Vec<PathControlPoint>,
    lazer_format: bool,
) {
    // Edge-case rules (to match stable).
    if kind == PathType::PERFECT_CURVE && !lazer_format {
        let end_point_len = if end_point.is_some() { 1 } else { 0 };

        if points.len() + end_point_len != 3 {
            kind = PathType::BEZIER;
        } else {
            let p2 = end_point.unwrap_or(points[2]);
            // osu-stable special-cased colinear perfect curves to a linear path.
            let cross = (points[1].y - points[0].y) * (p2.x - points[0].x)
                - (points[1].x - points[0].x) * (p2.y - points[0].y);
            if precision::almost_equals_f64(0.0, cross as f64) {
                kind = PathType::LINEAR;
            }
        }
    } else if kind == PathType::PERFECT_CURVE && lazer_format && points.len() + usize::from(end_point.is_some()) > 3 {
        kind = PathType::BEZIER;
    }

    // A path can have multiple implicit segments of the same type if there are
    // two sequential control points with the same position.
    let mut start_index = 0usize;
    let mut end_index = 0usize;

    let mut push_point = |pt: Vec2, ty: Option<PathType>, out: &mut Vec<PathControlPoint>| {
        // The merged list drops the second of a duplicated pair; boundary
        // points keep the segment type.
        out.push(PathControlPoint { position: pt, kind: ty });
    };

    let _ = &mut push_point;

    while {
        end_index += 1;
        end_index < points.len()
    } {
        if points[end_index] != points[end_index - 1] {
            continue;
        }

        // Legacy CATMULL sliders don't support multiple segments.
        if kind == PathType::CATMULL && end_index > 1 && !lazer_format {
            continue;
        }

        // The last control point of each segment is not allowed to start a new
        // implicit segment.
        if end_index == points.len() - 1 {
            continue;
        }

        // Emit [start_index, end_index): the point at end_index-1 becomes the
        // typed boundary; the duplicate at end_index is skipped.
        for (i, pt) in points[start_index..end_index].iter().enumerate() {
            let ty = if i == 0 && start_index == 0 {
                Some(kind)
            } else if i == end_index - start_index - 1 {
                Some(kind)
            } else {
                None
            };
            push_point(*pt, ty, output);
        }

        start_index = end_index + 1;
    }

    if start_index < end_index {
        for (i, pt) in points[start_index..end_index].iter().enumerate() {
            let ty = if i == 0 && start_index == 0 { Some(kind) } else { None };
            push_point(*pt, ty, output);
        }
    }
}

pub fn decode(content: &str) -> Result<Beatmap, String> {
    let mut version = 14i32;
    let mut mode = 0u8;
    let mut stack_leniency = 0.7f32;
    let mut difficulty = Difficulty::default();
    let mut has_approach_rate = false;

    let mut timing_points: Vec<TimingPoint> = Vec::new();
    let mut difficulty_points: Vec<DifficultyPoint> = Vec::new();
    let mut objects: Vec<ParsedObject> = Vec::new();
    let mut combo_colours: Vec<[u8; 3]> = Vec::new();
    let mut breaks: Vec<(f64, f64)> = Vec::new();
    let mut general = GeneralInfo::default();
    // Online IDs default to lazer's "unknown" -1 when the lines are absent.
    let mut metadata = BeatmapMetadata { beatmap_id: -1, beatmap_set_id: -1, ..Default::default() };
    let mut editor = EditorInfo::default();
    let mut background: Option<String> = None;
    let mut video: Option<VideoEvent> = None;

    // pending control point flush state
    let mut pending_time: Option<f64> = None;
    let mut pending_diff: Option<DifficultyPoint> = None;

    let mut section = String::new();
    let mut first_object = true;
    let mut last_object_is_spinner = false;

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches(['\r', '\n']).trim().to_string();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            flush_pending(&mut pending_time, &mut pending_diff, &mut timing_points, &mut difficulty_points);
            section = line[1..line.len() - 1].to_string();
            continue;
        }

        // Strip comments outside of metadata.
        let line = if section != "Metadata" {
            match line.find("//") {
                Some(idx) => line[..idx].trim().to_string(),
                None => line,
            }
        } else {
            line
        };
        if line.is_empty() {
            continue;
        }

        match section.as_str() {
            "General" => {
                let (key, value) = split_kv(&line);
                match key.as_str() {
                    "Mode" => mode = parse_i32(&value) as u8,
                    "StackLeniency" => stack_leniency = parse_f32(&value),
                    "AudioFilename" => general.audio_filename = Some(value),
                    "AudioLeadIn" => general.audio_lead_in = parse_f64(&value),
                    "PreviewTime" => {
                        let t = parse_f64(&value);
                        general.preview_time = if t < 0.0 { None } else { Some(t) };
                    }
                    "Countdown" => general.countdown = Some(parse_f64(&value)),
                    "SampleSet" => general.sample_set = Some(value),
                    "SampleVolume" => general.sample_volume = Some(parse_i32(&value).clamp(0, 100)),
                    "LetterboxInBreaks" => general.letterbox_in_breaks = parse_bool(&value),
                    "WidescreenStoryboard" => general.widescreen_storyboard = parse_bool(&value),
                    "SkinPreference" => general.skin_preference = Some(value),
                    _ => {}
                }
            }
            "Editor" => {
                let (key, value) = split_kv(&line);
                match key.as_str() {
                    "Bookmarks" => {
                        editor.bookmarks = value.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect();
                    }
                    "DistanceSpacing" => editor.distance_spacing = parse_f64(&value),
                    "BeatDivisor" => editor.beat_divisor = parse_i32(&value),
                    "GridSize" => editor.grid_size = parse_i32(&value),
                    "TimelineZoom" => editor.timeline_zoom = parse_f64(&value),
                    _ => {}
                }
            }
            "Metadata" => {
                let (key, value) = split_kv(&line);
                match key.as_str() {
                    "Title" => metadata.title = value,
                    "TitleUnicode" => metadata.title_unicode = value,
                    "Artist" => metadata.artist = value,
                    "ArtistUnicode" => metadata.artist_unicode = value,
                    "Creator" => metadata.creator = value,
                    "Version" => metadata.version = value,
                    "Source" => metadata.source = value,
                    "Tags" => metadata.tags = value.split_whitespace().map(str::to_string).collect(),
                    "BeatmapID" => metadata.beatmap_id = parse_i64(&value),
                    "BeatmapSetID" => metadata.beatmap_set_id = parse_i64(&value),
                    _ => {}
                }
            }
            "Difficulty" => {
                let (key, value) = split_kv(&line);
                match key.as_str() {
                    "HPDrainRate" => difficulty.hp = parse_f32(&value),
                    "CircleSize" => difficulty.cs = parse_f32(&value),
                    "OverallDifficulty" => difficulty.od = parse_f32(&value),
                    "ApproachRate" => {
                        difficulty.ar = parse_f32(&value);
                        has_approach_rate = true;
                    }
                    "SliderMultiplier" => difficulty.slider_multiplier = parse_f32(&value),
                    "SliderTickRate" => difficulty.slider_tick_rate = parse_f32(&value),
                    _ => {}
                }
            }
            "TimingPoints" => {
                let split: Vec<&str> = line.split(',').collect();
                let mut time = parse_f64(split[0]);
                if version < 5 {
                    time += EARLY_VERSION_TIMING_OFFSET;
                }

                let beat_length = parse_f64_allow_nan(split.get(1).copied().unwrap_or(""));
                let speed_multiplier = if beat_length < 0.0 { 100.0 / -beat_length } else { 1.0 };

                let timing_change = split
                    .get(6)
                    .map(|s| s.trim().starts_with('1'))
                    .unwrap_or(true);

                if pending_time != Some(time) {
                    flush_pending(&mut pending_time, &mut pending_diff, &mut timing_points, &mut difficulty_points);
                }

                if timing_change {
                    timing_points.push(TimingPoint { time, beat_length });
                }

                // Pending difficulty point; last one at the same timestamp wins.
                pending_time = Some(time);
                pending_diff = Some(DifficultyPoint {
                    time,
                    slider_velocity: speed_multiplier,
                    generate_ticks: !beat_length.is_nan(),
                });
            }
            "HitObjects" => {
                let obj = parse_hit_object_line(
                    &line,
                    version,
                    first_object,
                    last_object_is_spinner,
                )?;
                // Assign slider SV from the difficulty point active at this time.
                flush_pending(&mut pending_time, &mut pending_diff, &mut timing_points, &mut difficulty_points);
                let sv = difficulty_point_at_time(&difficulty_points, obj.start_time);
                let is_spinner = matches!(obj.raw, RawHitObject::Spinner { .. });
                objects.push(ParsedObject {
                    start_time: obj.start_time,
                    new_combo: obj.new_combo,
                    raw: obj.raw,
                    slider_velocity_multiplier: sv,
                    combo_offset: obj.combo_offset,
                });
                first_object = false;
                last_object_is_spinner = is_spinner;
            }
            "Colours" => {
                // "Combo1 : 255,128,64" (also "Combo" without number).
                let (key, value) = split_kv(&line);
                // Split at the first digit to get the alphabetic prefix
                // ("Combo1" / "Combo2" -> "Combo"); splitting at non-digits
                // would yield an empty first segment for letter-led keys.
                let key = key.split(|c: char| c.is_ascii_digit()).next().unwrap_or("").trim().to_string();
                if key == "Combo" {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() >= 3 {
                        let rgb = [
                            parse_i32(parts[0]).clamp(0, 255) as u8,
                            parse_i32(parts[1]).clamp(0, 255) as u8,
                            parse_i32(parts[2]).clamp(0, 255) as u8,
                        ];
                        combo_colours.push(rgb);
                    }
                }
            }
            "Events" => {
                // "2,start,end" (break period). `LegacyBeatmapDecoder`'s
                // `LegacyEventType.Break`: end clamps to >= start. Only the
                // break event matters to judgement; sprite/text events are
                // skipped.
                let split: Vec<&str> = line.split(',').collect();
                if split.first().map(|s| s.trim()) == Some("2") && split.len() >= 3 {
                    let start = parse_f64(split[1]);
                    let end = parse_f64(split[2]).max(start);
                    breaks.push((start, end));
                }
                // Background line `0,0,"bg.jpg",0,0`: the path may contain
                // commas, so pull it off the raw line; first non-empty wins.
                if background.is_none() && line.starts_with("0,0,") {
                    let rest = line[4..].trim_start_matches('"');
                    let end = rest.find('"').unwrap_or(rest.len());
                    let name = rest[..end].to_string();
                    if !name.is_empty() {
                        background = Some(name);
                    }
                }
                // Video line `Video,offset,"file.mp4"` / legacy `1,offset,...`.
                if video.is_none() && (line.starts_with("Video,") || line.starts_with("1,")) {
                    let rest = line.strip_prefix("Video,").or_else(|| line.strip_prefix("1,")).unwrap_or("");
                    let mut it = rest.splitn(2, ',');
                    let start_time = it.next().map(parse_f64).unwrap_or(0.0);
                    if let Some(path) = it.next() {
                        let path = path.trim().trim_matches('"').to_string();
                        if !path.is_empty() {
                            video = Some(VideoEvent { start_time, path });
                        }
                    }
                }
            }
            _ => {
                if section.is_empty() {
                    // File header: "osu file format vN".
                    if let Some(rest) = line.strip_prefix("osu file format v") {
                        version = parse_i32(rest.trim());
                    }
                }
            }
        }
    }

    flush_pending(&mut pending_time, &mut pending_diff, &mut timing_points, &mut difficulty_points);

    if mode != 0 {
        return Err(format!("only osu!standard beatmaps are supported (mode {})", mode));
    }

    // AR defaults to OD when missing.
    if !has_approach_rate {
        difficulty.ar = difficulty.od;
    }

    // Difficulty clamping (LegacyBeatmapDecoder).
    difficulty.hp = difficulty.hp.clamp(0.0, 10.0);
    difficulty.od = difficulty.od.clamp(0.0, 10.0);
    difficulty.ar = difficulty.ar.clamp(0.0, 10.0);
    difficulty.cs = difficulty.cs.clamp(0.0, 10.0);
    difficulty.slider_multiplier = difficulty.slider_multiplier.clamp(0.4, 3.6);
    difficulty.slider_tick_rate = difficulty.slider_tick_rate.clamp(0.5, 8.0);

    // Sort timing points & dedupe difficulty points (mirrors ControlPointInfo.Add redundancy).
    timing_points.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    let mut deduped: Vec<DifficultyPoint> = Vec::new();
    for pt in difficulty_points {
        // Last point at the same time replaces the previous one; redundant
        // (same SV + tick generation as currently active) points are dropped.
        if let Some(last) = deduped.last() {
            if last.time == pt.time {
                deduped.pop();
            }
        }
        let redundant = deduped
            .last()
            .map(|active| {
                active.slider_velocity == pt.slider_velocity && active.generate_ticks == pt.generate_ticks
            })
            .unwrap_or(false);
        if !redundant {
            deduped.push(pt);
        }
    }
    deduped.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    difficulty_points = deduped;

    // Stable sort hit objects by start time (LINQ OrderBy is stable).
    objects.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

    let sample_data = crate::samples::parse(content);

    Ok(Beatmap {
        version,
        stack_leniency,
        difficulty,
        timing_points,
        difficulty_points,
        objects,
        combo_colours,
        breaks,
        general,
        metadata,
        editor,
        background,
        video,
        sample_data,
    })
}

fn difficulty_point_at_time(points: &[DifficultyPoint], t: f64) -> f64 {
    let mut result = 1.0;
    for p in points {
        if p.time <= t {
            result = p.slider_velocity;
        } else {
            break;
        }
    }
    result
}

fn flush_pending(
    pending_time: &mut Option<f64>,
    pending_diff: &mut Option<DifficultyPoint>,
    timing_points: &mut Vec<TimingPoint>,
    difficulty_points: &mut Vec<DifficultyPoint>,
) {
    let _ = timing_points;
    if let (Some(_t), Some(dp)) = (*pending_time, *pending_diff) {
        difficulty_points.push(dp);
    }
    *pending_time = None;
    *pending_diff = None;
}

fn parse_f64_allow_nan(s: &str) -> f64 {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

fn split_kv(line: &str) -> (String, String) {
    match line.find(':') {
        Some(idx) => (line[..idx].trim().to_string(), line[idx + 1..].trim().to_string()),
        None => (line.trim().to_string(), String::new()),
    }
}

struct ParsedLine {
    start_time: f64,
    new_combo: bool,
    combo_offset: u32,
    raw: RawHitObject,
}

fn parse_hit_object_line(
    line: &str,
    version: i32,
    first_object: bool,
    last_object_is_spinner: bool,
) -> Result<ParsedLine, String> {
    let split: Vec<&str> = line.split(',').collect();
    let lazer_format = version >= FIRST_LAZER_VERSION;

    let pos = Vec2::new(parse_coord(split[0], lazer_format), parse_coord(split[1], lazer_format));

    let mut start_time = parse_f64(split[2]);
    if version < 5 {
        start_time += EARLY_VERSION_TIMING_OFFSET;
    }

    let obj_type = parse_i32(split[3]) as u32;
    let combo_flag = obj_type & (1 << 2) != 0;
    let combo_offset = (obj_type & (0b111 << 4)) >> 4;

    let new_combo = first_object || last_object_is_spinner || combo_flag;

    let raw = if obj_type & 1 != 0 {
        // Circle
        RawHitObject::Circle { pos }
    } else if obj_type & (1 << 1) != 0 {
        // Slider
        let mut repeat_count = parse_i32(split[6]);
        if repeat_count > 9000 {
            return Err("Repeat count is way too high".to_string());
        }
        repeat_count = (repeat_count - 1).max(0);

        let mut length: Option<f64> = None;
        if split.len() > 7 {
            let l = parse_f64(split[7]).max(0.0);
            if l != 0.0 {
                length = Some(l);
            }
        }

        let control_points = convert_path_string(split[5], pos, lazer_format);
        let path = SliderPath::new(control_points, length);

        // Zero-length slider heuristic: reset repeats.
        let mut repeat_count = repeat_count as usize;
        let path = path;
        if precision::almost_equals_f64(path.distance(), 0.0) {
            repeat_count = 0;
        }

        RawHitObject::Slider { pos, path, repeat_count }
    } else if obj_type & (1 << 3) != 0 {
        // Spinner
        let end_time = parse_f64(split[5]);
        let end_time = if version < 5 { end_time + EARLY_VERSION_TIMING_OFFSET } else { end_time };
        let duration = (end_time - start_time).max(0.0);
        RawHitObject::Spinner { end_time: start_time + duration }
    } else {
        return Err(format!("unknown hit object type: {}", split[3]));
    };

    Ok(ParsedLine { start_time, new_combo, combo_offset, raw })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata_general_editor_and_events() {
        let map = decode(
            "osu file format v14\n\
             [General]\n\
             AudioFilename: audio.mp3\n\
             AudioLeadIn: 1500\n\
             PreviewTime: -1\n\
             StackLeniency: 0.5\n\
             SampleSet: Soft\n\
             LetterboxInBreaks: 1\n\
             WidescreenStoryboard: 0\n\
             \n\
             [Editor]\n\
             Bookmarks: 1000,2000\n\
             DistanceSpacing: 1.5\n\
             BeatDivisor: 4\n\
             \n\
             [Metadata]\n\
             Title: Song Title // kept, metadata skips comment stripping\n\
             TitleUnicode: 曲名\n\
             Artist: Artist\n\
             Creator: Someone\n\
             Version: Insane\n\
             Tags: jpop rock\tvocal\n\
             BeatmapID: 123456\n\
             BeatmapSetID: 654321\n\
             \n\
             [Difficulty]\n\
             HPDrainRate:5\n\
             CircleSize:4\n\
             OverallDifficulty:6\n\
             SliderMultiplier:1.4\n\
             SliderTickRate:1\n\
             \n\
             [TimingPoints]\n\
             0,500,4,2,0,60,1,0\n\
             \n\
             [Events]\n\
             0,0,\"bg, file.jpg\",0,0\n\
             Video,1000,\"video.mp4\"\n\
             2,5000,6000\n\
             \n\
             [HitObjects]\n\
             256,192,1000,5,4,0:0:0:60:\n",
        )
        .unwrap();

        assert_eq!(map.general.audio_filename.as_deref(), Some("audio.mp3"));
        assert_eq!(map.general.audio_lead_in, 1500.0);
        assert_eq!(map.general.preview_time, None);
        assert_eq!(map.general.sample_set.as_deref(), Some("Soft"));
        assert_eq!(map.general.letterbox_in_breaks, Some(true));
        assert_eq!(map.general.widescreen_storyboard, Some(false));
        assert_eq!(map.stack_leniency, 0.5);

        assert_eq!(map.editor.bookmarks, vec![1000.0, 2000.0]);
        assert_eq!(map.editor.distance_spacing, 1.5);
        assert_eq!(map.editor.beat_divisor, 4);

        assert_eq!(map.metadata.title, "Song Title // kept, metadata skips comment stripping");
        assert_eq!(map.metadata.title_unicode, "曲名");
        assert_eq!(map.metadata.creator, "Someone");
        assert_eq!(map.metadata.version, "Insane");
        assert_eq!(map.metadata.tags, vec!["jpop", "rock", "vocal"]);
        assert_eq!(map.metadata.beatmap_id, 123456);
        assert_eq!(map.metadata.beatmap_set_id, 654321);

        // Quoted path containing a comma survives.
        assert_eq!(map.background.as_deref(), Some("bg, file.jpg"));
        assert_eq!(map.video.as_ref().map(|v| v.path.as_str()), Some("video.mp4"));
        assert_eq!(map.video.as_ref().map(|v| v.start_time), Some(1000.0));
        assert_eq!(map.breaks, vec![(5000.0, 6000.0)]);

        // Sample data: soft bank + volume 60 from the timing point, and the
        // circle's trailing hitSample.
        assert_eq!(map.sample_data.points.len(), 1);
        assert_eq!(map.sample_data.points[0].volume, 60);
        assert_eq!(map.sample_data.points[0].bank, crate::samples::SampleBank::Soft);
        assert_eq!(map.sample_data.objects.len(), 1);
        assert_eq!(map.sample_data.objects[0].bank.volume, 60);
    }

    #[test]
    fn missing_metadata_defaults_to_unknown_ids() {
        let map = decode("osu file format v14\n[HitObjects]\n256,192,1000,5,0,0:0:0:0:\n").unwrap();
        assert_eq!(map.metadata.beatmap_id, -1);
        assert_eq!(map.metadata.beatmap_set_id, -1);
        assert_eq!(map.metadata.title, "");
        assert!(map.background.is_none());
        assert!(map.video.is_none());
        assert!(map.sample_data.points.is_empty());
    }
}
