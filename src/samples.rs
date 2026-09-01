//! `.osu` sample-side data: per-timing-point sample banks/volumes and
//! per-hitobject hitSample/edgeSets overrides, mirroring lazer's
//! `ConvertHitObjectParser` sample bank reading (`readCustomSampleBanks`).
//!
//! This is the data half of hitsound synthesis; the playback/resolution
//! rules (leniency offsets, node samples, volumes) live with the audio
//! consumer.

/// `LegacySampleBank`: the three sample banks a point or object can select.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SampleBank {
    Normal,
    Soft,
    Drum,
}

impl SampleBank {
    /// `(LegacySampleBank)int`; 0 (None) and invalid values mean "not
    /// specified" for object-level banks, and Normal for timing points.
    pub fn from_legacy(v: i64) -> Option<SampleBank> {
        match v {
            1 => Some(SampleBank::Normal),
            2 => Some(SampleBank::Soft),
            3 => Some(SampleBank::Drum),
            _ => None,
        }
    }

    /// Bank directory name in a skin (`normal/`, `soft/`, `drum/`).
    pub fn as_str(self) -> &'static str {
        match self {
            SampleBank::Normal => "normal",
            SampleBank::Soft => "soft",
            SampleBank::Drum => "drum",
        }
    }
}

/// A timing point's sample settings (`LegacySampleControlPoint`).
#[derive(Clone, Copy, Debug)]
pub struct SamplePoint {
    pub time: f64,
    pub bank: SampleBank,
    /// 0-100.
    pub volume: i32,
}

/// `ConvertHitObjectParser.SampleBankInfo`: banks/volume read from a
/// hitobject's trailing hitSample (or a slider's edgeSets entry).
#[derive(Clone, Default, Debug)]
pub struct SampleBankInfo {
    /// Bank for hitnormal; `None` = inherit from the control point.
    pub normal: Option<SampleBank>,
    /// Bank for additions; `None` = same as `normal`.
    pub additions: Option<SampleBank>,
    /// 0-100; 0 = inherit from the control point.
    pub volume: i32,
}

/// Sample-side view of one hit object.
#[derive(Default)]
pub struct SampleObject {
    pub start_time: f64,
    /// Circle: start time; spinner: end time; slider: unused (node sample
    /// points are resolved from the processed object's duration).
    pub end_time: f64,
    /// Slider only: the .osu repeat field (span count).
    pub span_count: usize,
    /// HitSound bitmask: 2 whistle, 4 finish, 8 clap.
    pub sound_type: u8,
    pub bank: SampleBankInfo,
    /// Per-node (head, repeats..., tail) sound types / banks.
    pub node_types: Vec<u8>,
    pub node_banks: Vec<SampleBankInfo>,
}

/// Everything the .osu says about samples: one point per `[TimingPoints]`
/// line plus one entry per hit object, sorted to line up with the decoded
/// beatmap's objects.
#[derive(Default)]
pub struct SampleData {
    pub points: Vec<SamplePoint>,
    pub objects: Vec<SampleObject>,
}

/// `readCustomSampleBanks`. `banks_only` mirrors the slider object-level
/// call, where the trailing hitSample contributes banks but no volume.
fn read_custom_sample_banks(s: &str, info: &mut SampleBankInfo, banks_only: bool) {
    let split: Vec<&str> = s.split(':').collect();
    let parse = |v: Option<&&str>| -> i64 {
        v.and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0)
    };
    info.normal = SampleBank::from_legacy(parse(split.first()));
    let add = SampleBank::from_legacy(parse(split.get(1)));
    info.additions = add.or(info.normal);
    if !banks_only && split.len() > 3 {
        info.volume = parse(split.get(3)).max(0) as i32;
    }
}

/// Parse the sample-side data out of a `.osu` file's raw text. Deliberately
/// its own pass (not folded into [`crate::beatmap::decode`]'s section loop):
/// timing points here keep their raw file time (no pre-v5 +24ms offset) and
/// every `[TimingPoints]` line yields a point, matching lazer's separate
/// `LegacySampleControlPoint` grouping.
pub fn parse(content: &str) -> SampleData {
    let mut default_bank = SampleBank::Normal;
    let mut default_volume = 100i32;
    let mut points: Vec<SamplePoint> = Vec::new();
    let mut objects: Vec<SampleObject> = Vec::new();
    let mut section = "";

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            continue;
        }

        match section {
            "General" => {
                let mut pair = line.splitn(2, ':');
                let key = pair.next().unwrap_or("").trim();
                let value = pair.next().unwrap_or("").trim();
                match key {
                    "SampleSet" => {
                        default_bank = match value.to_ascii_lowercase().as_str() {
                            "soft" => SampleBank::Soft,
                            "drum" => SampleBank::Drum,
                            _ => SampleBank::Normal, // normal + none + numeric 0
                        };
                    }
                    "SampleVolume" => {
                        default_volume = value.trim().parse().unwrap_or(default_volume);
                    }
                    _ => {}
                }
            }
            "TimingPoints" => {
                let split: Vec<&str> = line.split(',').collect();
                let time = split.first().and_then(|s| s.trim().parse::<f64>().ok());
                let Some(time) = time else { continue };
                // Fields 4-6: sampleSet, sampleSetIndex, volume. Missing
                // fields fall back to the [General] defaults.
                let bank = split
                    .get(3)
                    .and_then(|s| s.trim().parse::<i64>().ok())
                    .and_then(SampleBank::from_legacy)
                    .unwrap_or(default_bank);
                let volume = split
                    .get(5)
                    .and_then(|s| s.trim().parse::<i64>().ok())
                    .unwrap_or(default_volume as i64)
                    .clamp(0, 100) as i32;
                let point = SamplePoint { time, bank, volume };
                // Same-time lines: the last one wins (control point groups
                // replace, non-redundant later additions override).
                if points.last().map(|p| p.time == time).unwrap_or(false) {
                    points.pop();
                }
                points.push(point);
            }
            "HitObjects" => {
                let split: Vec<&str> = line.split(',').collect();
                if split.len() < 5 {
                    continue;
                }
                let start_time = split[2].trim().parse::<f64>().unwrap_or(0.0);
                let obj_type = split[3].trim().parse::<i64>().unwrap_or(0);
                let sound_type = split[4].trim().parse::<i64>().unwrap_or(0).max(0) as u8;
                let mut bank = SampleBankInfo::default();

                if obj_type & 1 != 0 {
                    // Circle: x,y,time,type,hitSound,hitSample
                    if let Some(s) = split.get(5) {
                        read_custom_sample_banks(s, &mut bank, false);
                    }
                    objects.push(SampleObject {
                        start_time,
                        end_time: start_time,
                        span_count: 0,
                        sound_type,
                        bank,
                        node_types: Vec::new(),
                        node_banks: Vec::new(),
                    });
                } else if obj_type & 2 != 0 {
                    // Slider: ...,path,repeats,pixelLength,edgeSounds,
                    // edgeSets,hitSample (hitSample is banks-only).
                    if let Some(s) = split.get(10) {
                        read_custom_sample_banks(s, &mut bank, true);
                    }
                    let span_count = split.get(5 + 1).and_then(|s| s.trim().parse::<usize>().ok()).unwrap_or(1);
                    let nodes = span_count + 1;
                    let mut node_banks = vec![bank.clone(); nodes];
                    if let Some(sets) = split.get(9).filter(|s| !s.is_empty()) {
                        for (i, set) in sets.split('|').enumerate() {
                            if i >= nodes {
                                break;
                            }
                            read_custom_sample_banks(set, &mut node_banks[i], false);
                        }
                    }
                    let mut node_types = vec![sound_type; nodes];
                    if let Some(adds) = split.get(8).filter(|s| !s.is_empty()) {
                        for (i, add) in adds.split('|').enumerate() {
                            if i >= nodes {
                                break;
                            }
                            if let Ok(v) = add.trim().parse::<i64>() {
                                node_types[i] = v.max(0) as u8;
                            }
                        }
                    }
                    objects.push(SampleObject {
                        start_time,
                        end_time: start_time,
                        span_count,
                        sound_type,
                        bank,
                        node_types,
                        node_banks,
                    });
                } else if obj_type & 8 != 0 {
                    // Spinner: x,y,time,type,hitSound,endTime,hitSample
                    let end_time = split.get(5).and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(start_time);
                    if let Some(s) = split.get(6) {
                        read_custom_sample_banks(s, &mut bank, false);
                    }
                    objects.push(SampleObject {
                        start_time,
                        end_time: end_time.max(start_time),
                        span_count: 0,
                        sound_type,
                        bank,
                        node_types: Vec::new(),
                        node_banks: Vec::new(),
                    });
                }
            }
            _ => {}
        }
    }

    // Match the decoder's ordering so indices line up with the processed
    // objects: stable sort by start time.
    objects.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    SampleData { points, objects }
}
