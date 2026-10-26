use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use pgs::{PgsDisplaySet, PgsDisplaySetState};
use tesseract::plumbing::leptonica_plumbing::Pix;
use tesseract::PageSegMode;

use crate::helpers::{get_tiff_stream, ManagedTesseract};
use crate::language::{is_selectable_ocr_language, recognition_data_path};

/// Unambiguous full language names (English + native) mapped to Tesseract ISO-639-2/script codes.
const FULL_LANGUAGE_NAMES: &[(&[&str], &str)] = &[
    (&["english"], "eng"),
    (&["german", "deutsch"], "deu"),
    (&["croatian", "hrvatski"], "hrv"),
    (&["bosnian", "bosanski"], "bos"),
    (&["serbian", "srpski"], "srp"),
    (&["slovenian", "slovenski", "slovene"], "slv"),
    (&["french", "francais", "français"], "fra"),
    (&["spanish", "espanol", "español", "castellano"], "spa"),
    (&["italian", "italiano"], "ita"),
    (&["portuguese", "portugues", "português"], "por"),
    (&["dutch", "nederlands", "flemish"], "nld"),
    (&["russian"], "rus"),
    (&["japanese"], "jpn"),
    (&["chinese", "mandarin", "cantonese"], "chi_sim"),
    (&["polish", "polski"], "pol"),
    (&["swedish", "svenska"], "swe"),
    (&["norwegian", "norsk"], "nor"),
    (&["danish", "dansk"], "dan"),
    (&["finnish", "suomi"], "fin"),
    (&["czech", "cesky", "český"], "ces"),
    (&["slovak", "slovensky", "slovenský"], "slk"),
    (&["greek"], "ell"),
    (&["turkish", "turkce", "türkçe"], "tur"),
    (&["arabic"], "ara"),
    (&["korean"], "kor"),
    (&["ukrainian"], "ukr"),
    (&["bulgarian"], "bul"),
    (&["romanian", "romana", "română"], "ron"),
    (&["hungarian", "magyar"], "hun"),
    (&["hebrew"], "heb"),
    (&["vietnamese"], "vie"),
    (&["thai"], "tha"),
    (&["indonesian"], "ind"),
];

/// 3-letter ISO-639-2 (T & B) and Tesseract composite codes.
const ISO3_LANGUAGE_CODES: &[(&[&str], &str)] = &[
    (&["eng"], "eng"),
    (&["deu", "ger"], "deu"),
    (&["hrv", "cro", "scr"], "hrv"),
    (&["bos"], "bos"),
    (&["srp", "scc"], "srp"),
    (&["srp_latn"], "srp_latn"),
    (&["slv"], "slv"),
    (&["fra", "fre"], "fra"),
    (&["spa"], "spa"),
    (&["ita"], "ita"),
    (&["por"], "por"),
    (&["nld", "dut"], "nld"),
    (&["rus"], "rus"),
    (&["jpn"], "jpn"),
    (&["chi", "zho", "chi_sim", "chs"], "chi_sim"),
    (&["chi_tra", "cht"], "chi_tra"),
    (&["pol"], "pol"),
    (&["swe"], "swe"),
    (&["nor", "nob", "nno"], "nor"),
    (&["dan"], "dan"),
    (&["fin"], "fin"),
    (&["ces", "cze"], "ces"),
    (&["slk", "slo"], "slk"),
    (&["ell", "gre"], "ell"),
    (&["tur"], "tur"),
    (&["ara"], "ara"),
    (&["kor"], "kor"),
    (&["ukr"], "ukr"),
    (&["bul"], "bul"),
    (&["ron", "rum"], "ron"),
    (&["hun"], "hun"),
    (&["heb"], "heb"),
    (&["vie"], "vie"),
    (&["tha"], "tha"),
    (&["ind"], "ind"),
];

/// 2-letter ISO-639-1 codes (only accepted in explicit subtitle tag suffix position).
const ISO2_LANGUAGE_CODES: &[(&str, &str)] = &[
    ("en", "eng"),
    ("de", "deu"),
    ("hr", "hrv"),
    ("bs", "bos"),
    ("sr", "srp"),
    ("sl", "slv"),
    ("fr", "fra"),
    ("es", "spa"),
    ("it", "ita"),
    ("pt", "por"),
    ("nl", "nld"),
    ("ru", "rus"),
    ("ja", "jpn"),
    ("zh", "chi_sim"),
    ("pl", "pol"),
    ("sv", "swe"),
    ("no", "nor"),
    ("da", "dan"),
    ("fi", "fin"),
    ("cs", "ces"),
    ("sk", "slk"),
    ("el", "ell"),
    ("tr", "tur"),
    ("ar", "ara"),
    ("ko", "kor"),
    ("uk", "ukr"),
    ("bg", "bul"),
    ("ro", "ron"),
    ("hu", "hun"),
    ("he", "heb"),
    ("vi", "vie"),
    ("th", "tha"),
    ("id", "ind"),
];

/// Common 2-letter words in Croatian/English/European movie titles ("ja" = "I" in Croatian,
/// "it", "no", "de", "el", "id", "he") that must ONLY match when preceded by `.`, `[`, or `(`.
const AMBIGUOUS_ISO2_WORDS: &[&str] = &["ja", "it", "no", "de", "el", "id", "he", "en", "es", "tr"];

/// Trailing subtitle flags that may follow the language tag before `.sup`.
const SUBTITLE_MODIFIERS: &[&str] = &[
    "forced",
    "sdh",
    "hi",
    "cc",
    "hearing",
    "impaired",
    "signs",
    "songs",
    "full",
    "default",
    "pgs",
    "sup",
    "sub",
    "subs",
    "subtitle",
    "subtitles",
];

fn is_subtitle_modifier(token_lower: &str) -> bool {
    SUBTITLE_MODIFIERS.contains(&token_lower)
        || (token_lower.starts_with("track") && token_lower[5..].chars().all(|c| c.is_ascii_digit()))
        || (token_lower.starts_with("stream") && token_lower[6..].chars().all(|c| c.is_ascii_digit()))
}

/// Tries to determine the subtitle language from filename conventions (e.g. `movie.hrv.sup`,
/// `show.S01E01.deu.sup`, `video_Croatian.sup`) without misclassifying movie title words.
pub fn detect_language_from_path(path: &Path) -> Option<&'static str> {
    let file_stem = path.file_stem()?.to_str()?;
    detect_language_from_filename(file_stem)
}

#[derive(Clone, Copy, Debug)]
struct FilenameToken<'a> {
    delim_before: Option<char>,
    text: &'a str,
}

fn tokenize_filename(name: &str) -> Vec<FilenameToken<'_>> {
    let is_delim = |c: char| matches!(c, '.' | '_' | '-' | ' ' | '[' | ']' | '(' | ')');
    let mut tokens = Vec::new();
    let mut current_start: Option<usize> = None;
    let mut last_delim: Option<char> = None;

    for (idx, ch) in name.char_indices() {
        if is_delim(ch) {
            if let Some(start) = current_start.take() {
                let slice = &name[start..idx];
                if !slice.is_empty() {
                    tokens.push(FilenameToken { delim_before: last_delim, text: slice });
                }
            }
            // Prefer structural tag delimiters (`.`, `[`, `(`) when multiple delimiters are adjacent (e.g. ` [Spanish]`).
            last_delim = match (last_delim, ch) {
                (Some(prev @ ('.' | '[' | '(')), ' ') => Some(prev),
                _ => Some(ch),
            };
        } else if current_start.is_none() {
            current_start = Some(idx);
        }
    }

    if let Some(start) = current_start {
        let slice = &name[start..];
        if !slice.is_empty() {
            tokens.push(FilenameToken { delim_before: last_delim, text: slice });
        }
    }

    tokens
}

/// Parses filename trailing subtitle tags for known language codes or full language names.
pub fn detect_language_from_filename(name: &str) -> Option<&'static str> {
    // Allow composite tags containing `_` at the end (e.g. `movie.srp_latn` or `movie.chi_sim`)
    let lower_trimmed = name.trim().to_lowercase();
    for composite in ["srp_latn", "chi_sim", "chi_tra"] {
        if lower_trimmed.ends_with(&format!(".{composite}"))
            || lower_trimmed.ends_with(&format!("_{composite}"))
            || lower_trimmed.ends_with(&format!("-{composite}"))
            || lower_trimmed.ends_with(&format!("[{composite}]"))
        {
            return Some(composite);
        }
    }

    let tokens = tokenize_filename(name);
    if tokens.is_empty() {
        return None;
    }

    // Strip trailing subtitle modifier flags (e.g. `.forced`, `.sdh`, `.pgs`, `.track3`).
    let mut end = tokens.len();
    while end > 0 && is_subtitle_modifier(&tokens[end - 1].text.to_lowercase()) {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let candidate = tokens[end - 1];
    let lower = candidate.text.to_lowercase();

    // 1. Full unambiguous language names (e.g. `Croatian`, `Hrvatski`, `German`, `Spanish`)
    for (names, code) in FULL_LANGUAGE_NAMES {
        if names.contains(&lower.as_str()) {
            return Some(code);
        }
    }

    // ISO short codes require at least 2 tokens (a title stem + a tag) and a non-space tag separator.
    if end < 2 {
        return None;
    }
    let delim = candidate.delim_before.unwrap_or(' ');
    if delim == ' ' {
        return None;
    }

    // 2. 3-letter ISO-639-2 codes (e.g. `.hrv`, `_eng`, `-deu`, `[spa]`)
    for (codes, target) in ISO3_LANGUAGE_CODES {
        if codes.contains(&lower.as_str()) {
            return Some(target);
        }
    }

    // 3. 2-letter ISO-639-1 codes (e.g. `.hr`, `.en`, `[de]`)
    for &(iso2, target) in ISO2_LANGUAGE_CODES {
        if lower == iso2 {
            if AMBIGUOUS_ISO2_WORDS.contains(&iso2) && !matches!(delim, '.' | '[' | '(') {
                return None;
            }
            return Some(target);
        }
    }

    None
}

// =============================================================================
// TWO-STAGE VISUAL + LINGUISTIC SUBTITLE LANGUAGE DETECTION
// =============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ScriptFamily {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Japanese,
    Chinese,
    Korean,
    Other,
}

fn language_script_family(code: &str) -> ScriptFamily {
    match code {
        "jpn" => ScriptFamily::Japanese,
        "chi_sim" | "chi_tra" => ScriptFamily::Chinese,
        "kor" => ScriptFamily::Korean,
        "rus" | "ukr" | "srp" | "bul" | "bel" | "mkd" | "kaz" | "kir" | "mon" | "tgk" | "tat" | "aze_cyrl" | "uzb_cyrl" => {
            ScriptFamily::Cyrillic
        }
        "ell" | "grc" => ScriptFamily::Greek,
        "ara" | "fas" | "urd" | "pus" | "uig" | "kur_ara" | "snd" | "div" | "syr" => ScriptFamily::Arabic,
        "heb" | "yid" => ScriptFamily::Hebrew,
        "hin" | "mar" | "nep" | "san" | "ben" | "asm" | "pan" | "guj" | "ori" | "tam" | "tel" | "kan" | "mal" | "sin" | "tha"
        | "lao" | "mya" | "khm" | "bod" | "dzo" | "kat" | "kat_old" | "hye" | "amh" | "tir" | "chr" | "iku" => {
            ScriptFamily::Other
        }
        _ => ScriptFamily::Latin,
    }
}

#[derive(Clone, Debug)]
struct OcrSampleRun {
    text: String,
    mean_conf: f32,
}

fn run_ocr_on_tiff_samples(lang: &str, tiff_samples: &[Vec<u8>], cancel: &AtomicBool) -> Option<OcrSampleRun> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let data_path = recognition_data_path(lang).unwrap_or(None);
    let mut api = ManagedTesseract::new(data_path.as_deref(), lang)
        .inspect_err(|e| log::warn!("Language probe skipped {lang}: {e}"))
        .ok()?;

    let mut combined = String::new();
    let mut weighted_conf_sum = 0.0_f32;
    let mut total_weight = 0.0_f32;

    for tiff in tiff_samples {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let Ok(pix) = Pix::read_mem(tiff) else {
            return None;
        };
        let mut cue_text = String::new();
        let mut cue_conf = 0_i32;

        for mode in [PageSegMode::PsmSingleBlock, PageSegMode::PsmAuto] {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let tess = api.as_mut();
            tess.set_page_seg_mode(mode.as_tess_page_seg_mode());
            tess.set_image_2(&pix);
            tess.set_source_resolution(300);
            if tess.recognize().is_err() {
                return None;
            }
            cue_conf = tess.mean_text_conf().clamp(0, 100);
            if let Ok(raw) = tess.get_utf8_text() {
                let t = raw.as_ref().to_string_lossy().into_owned();
                if !t.trim().is_empty() {
                    cue_text = t;
                    break;
                }
            }
        }

        let trimmed = cue_text.trim();
        if !trimmed.is_empty() {
            let char_weight = trimmed.chars().filter(|c| !c.is_whitespace()).count() as f32;
            weighted_conf_sum += (cue_conf as f32) * char_weight;
            total_weight += char_weight;
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(trimmed);
        }
    }

    if total_weight <= 0.0 || cancel.load(Ordering::Relaxed) {
        return None;
    }

    Some(OcrSampleRun { text: combined, mean_conf: weighted_conf_sum / total_weight })
}

fn is_latin_alpha(c: char) -> bool {
    c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&c) || ('\u{1E00}'..='\u{1EFF}').contains(&c)
}

fn is_cyrillic_alpha(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}

fn is_distinct_cyrillic(c: char) -> bool {
    let lower = c.to_lowercase().next().unwrap_or(c);
    // Excludes Latin visual homoglyphs (а, е, о, р, с, у, х, і, ј)
    is_cyrillic_alpha(lower) && !matches!(lower, 'а' | 'е' | 'о' | 'р' | 'с' | 'у' | 'х' | 'і' | 'ј')
}

fn is_hiragana(c: char) -> bool {
    ('\u{3040}'..='\u{309F}').contains(&c)
}

fn is_katakana(c: char) -> bool {
    ('\u{30A0}'..='\u{30FF}').contains(&c)
}

fn is_cjk_ideograph(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c)
}

fn is_hangul(c: char) -> bool {
    ('\u{AC00}'..='\u{D7AF}').contains(&c)
}

/// Evaluates how genuinely `run` matches `family`, rejecting cross-script hallucinations
/// (such as `jpn` slicing Latin strokes into low-confidence isolated Kana/Kanji glyphs).
fn evaluate_script_fit(family: ScriptFamily, run: &OcrSampleRun) -> f32 {
    if run.mean_conf < 52.0 {
        return 0.0;
    }

    match family {
        ScriptFamily::Latin => {
            let latin_chars = run.text.chars().filter(|&c| is_latin_alpha(c)).count();
            if latin_chars < 4 {
                return 0.0;
            }
            let words: Vec<&str> =
                run.text.split(|c: char| !is_latin_alpha(c) && c != '\'' && c != '’').filter(|w| !w.is_empty()).collect();
            if words.is_empty() {
                return 0.0;
            }
            let multi_letter = words.iter().filter(|w| w.chars().count() >= 2).count();
            let multi_ratio = multi_letter as f32 / words.len() as f32;
            run.mean_conf * (0.45 + 0.55 * multi_ratio) * (latin_chars.min(60) as f32 / 60.0).max(0.35)
        }
        ScriptFamily::Cyrillic => {
            let cyr_chars = run.text.chars().filter(|&c| is_cyrillic_alpha(c)).count();
            let distinct = run.text.chars().filter(|&c| is_distinct_cyrillic(c)).count();
            if cyr_chars < 4 || distinct * 4 < cyr_chars {
                return 0.0;
            }
            let words: Vec<&str> = run.text.split(|c: char| !is_cyrillic_alpha(c)).filter(|w| !w.is_empty()).collect();
            let multi_ratio = if words.is_empty() {
                0.0
            } else {
                words.iter().filter(|w| w.chars().count() >= 2).count() as f32 / words.len() as f32
            };
            run.mean_conf * (0.45 + 0.55 * multi_ratio) * (cyr_chars.min(60) as f32 / 60.0).max(0.35)
        }
        ScriptFamily::Japanese => {
            // Anti-hallucination guard: real Japanese subtitles have high confidence, contiguous
            // Kana/Kanji compounds (not spaces between every stroke), and Hiragana grammar.
            if run.mean_conf < 68.0 {
                return 0.0;
            }
            let hira = run.text.chars().filter(|&c| is_hiragana(c)).count();
            let kata = run.text.chars().filter(|&c| is_katakana(c)).count();
            let kanji = run.text.chars().filter(|&c| is_cjk_ideograph(c)).count();
            let jp_total = hira + kata + kanji;
            if jp_total < 3 || (hira == 0 && kanji < 3) {
                return 0.0;
            }
            let chars: Vec<char> = run.text.chars().collect();
            let is_jp = |c: char| is_hiragana(c) || is_katakana(c) || is_cjk_ideograph(c);
            let contiguous_pairs = chars.windows(2).filter(|w| is_jp(w[0]) && is_jp(w[1])).count();
            if contiguous_pairs * 3 < jp_total {
                return 0.0;
            }
            run.mean_conf * (jp_total.min(40) as f32 / 40.0).max(0.4)
        }
        ScriptFamily::Chinese => {
            if run.mean_conf < 68.0 {
                return 0.0;
            }
            let han = run.text.chars().filter(|&c| is_cjk_ideograph(c)).count();
            if han < 3 {
                return 0.0;
            }
            let chars: Vec<char> = run.text.chars().collect();
            let contiguous_pairs = chars.windows(2).filter(|w| is_cjk_ideograph(w[0]) && is_cjk_ideograph(w[1])).count();
            if contiguous_pairs * 3 < han {
                return 0.0;
            }
            run.mean_conf * (han.min(40) as f32 / 40.0).max(0.4)
        }
        ScriptFamily::Korean => {
            if run.mean_conf < 68.0 {
                return 0.0;
            }
            let hangul = run.text.chars().filter(|&c| is_hangul(c)).count();
            if hangul < 3 {
                return 0.0;
            }
            run.mean_conf * (hangul.min(40) as f32 / 40.0).max(0.4)
        }
        ScriptFamily::Greek => {
            let gr = run.text.chars().filter(|&c| ('\u{0370}'..='\u{03FF}').contains(&c)).count();
            if gr < 4 || run.mean_conf < 65.0 {
                return 0.0;
            }
            run.mean_conf * (gr.min(50) as f32 / 50.0).max(0.4)
        }
        ScriptFamily::Arabic => {
            let ar = run.text.chars().filter(|&c| ('\u{0600}'..='\u{06FF}').contains(&c)).count();
            if ar < 4 || run.mean_conf < 65.0 {
                return 0.0;
            }
            run.mean_conf * (ar.min(50) as f32 / 50.0).max(0.4)
        }
        ScriptFamily::Hebrew => {
            let he = run.text.chars().filter(|&c| ('\u{0590}'..='\u{05FF}').contains(&c)).count();
            if he < 4 || run.mean_conf < 65.0 {
                return 0.0;
            }
            run.mean_conf * (he.min(50) as f32 / 50.0).max(0.4)
        }
        ScriptFamily::Other => {
            let non_ascii = run.text.chars().filter(|c| c.is_alphabetic() && !c.is_ascii()).count();
            if non_ascii < 4 || run.mean_conf < 70.0 {
                return 0.0;
            }
            run.mean_conf * 0.75
        }
    }
}

struct LangLinguisticProfile {
    code: &'static str,
    unique_chars: &'static [char],
    shared_chars: &'static [char],
    forbidden_chars: &'static [char],
    signature_words: &'static [&'static str],
    common_words: &'static [&'static str],
}

const LATIN_PROFILES: &[LangLinguisticProfile] = &[
    LangLinguisticProfile {
        code: "hrv",
        unique_chars: &['đ'],
        shared_chars: &['č', 'ć', 'ž', 'š'],
        forbidden_chars: &['ě', 'ř', 'ů', 'ł', 'ą', 'ę', 'ä', 'ö', 'ü', 'ß', 'ñ', 'ă', 'ș', 'ț', 'ğ', 'ı', 'ő', 'ű'],
        signature_words: &[
            "što",
            "ovdje",
            "gdje",
            "uvijek",
            "vrijeme",
            "lijepo",
            "prije",
            "poslije",
            "čovjek",
            "tko",
            "netko",
            "nitko",
            "ništa",
            "nešto",
            "zašto",
            "možda",
            "hoćeš",
            "hoću",
            "neću",
            "ćemo",
            "ćete",
            "općenito",
            "tisuća",
            "tjedan",
            "obitelj",
            "uopće",
            "večeras",
            "kući",
            "doći",
            "ići",
            "reći",
            "naći",
            "pomoći",
            "sto",
            "zasto",
            "nista",
            "nesto",
            "covjek",
        ],
        common_words: &[
            "je", "su", "sam", "smo", "ste", "nije", "nisam", "nisu", "kako", "kada", "kad", "ali", "ili", "nego", "koji",
            "koja", "koje", "ovo", "toga", "tako", "samo", "više", "već", "još", "sve", "svi", "moj", "moja", "tvoj", "dobro",
            "hvala", "molim", "idemo", "treba", "može", "možeš", "mogu", "znam", "znaš", "mislim", "rekao", "rekla", "bio",
            "bila", "biti", "ne", "da", "na", "za", "od", "iz", "sa", "kod",
        ],
    },
    LangLinguisticProfile {
        code: "srp_latn",
        unique_chars: &[],
        shared_chars: &['č', 'ć', 'ž', 'š', 'đ'],
        forbidden_chars: &['ě', 'ř', 'ů', 'ł', 'ą', 'ę', 'ä', 'ö', 'ü', 'ß', 'ñ'],
        signature_words: &[
            "šta", "ovde", "gde", "uvek", "vreme", "lepo", "posle", "čovek", "neko", "niko", "hiljada", "nedelja", "porodica",
            "uopšte", "možda", "neću", "hoćeš",
        ],
        common_words: &[
            "je", "su", "sam", "smo", "ste", "nije", "nisam", "nisu", "kako", "zašto", "ništa", "nešto", "ali", "ili", "samo",
            "tako", "dobro", "hvala", "molim", "znam", "mislim",
        ],
    },
    LangLinguisticProfile {
        code: "bos",
        unique_chars: &[],
        shared_chars: &['č', 'ć', 'ž', 'š', 'đ'],
        forbidden_chars: &['ě', 'ř', 'ů', 'ł', 'ą', 'ę', 'ä', 'ö', 'ü', 'ß', 'ñ'],
        signature_words: &["lahko", "kahva", "mahala", "amidža", "daidža", "baš", "ovdje", "gdje", "uvijek"],
        common_words: &["je", "su", "sam", "smo", "nije", "nisam", "kako", "zašto", "ali", "samo", "dobro"],
    },
    LangLinguisticProfile {
        code: "slv",
        unique_chars: &[],
        shared_chars: &['č', 'š', 'ž'],
        forbidden_chars: &['ć', 'đ', 'ě', 'ř', 'ł', 'ä', 'ö', 'ü', 'ß'],
        signature_words: &[
            "kaj",
            "zakaj",
            "tukaj",
            "kje",
            "vedno",
            "lahko",
            "ampak",
            "tudi",
            "zelo",
            "zdaj",
            "bova",
            "boste",
            "niso",
            "nisem",
            "prosim",
            "hvala",
            "živjo",
            "nasvidenje",
        ],
        common_words: &["je", "so", "sem", "smo", "ste", "ni", "kako", "kdaj", "ali", "samo", "dobro", "vse"],
    },
    LangLinguisticProfile {
        code: "eng",
        unique_chars: &[],
        shared_chars: &[],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ä', 'ö', 'ü', 'ß', 'ñ', 'ł', 'ą', 'ę', 'ě', 'ř', 'ă', 'ș', 'ț', 'ğ', 'ı'],
        signature_words: &[
            "the",
            "and",
            "that",
            "have",
            "with",
            "this",
            "what",
            "your",
            "from",
            "they",
            "would",
            "there",
            "their",
            "about",
            "could",
            "should",
            "because",
            "something",
            "nothing",
            "everything",
            "don't",
            "it's",
            "i'm",
            "can't",
            "won't",
            "didn't",
            "you're",
            "we're",
        ],
        common_words: &[
            "you", "for", "not", "are", "was", "were", "his", "her", "out", "who", "get", "has", "him", "how", "know", "take",
            "into", "just", "think", "come", "going", "right", "here", "where", "when", "why",
        ],
    },
    LangLinguisticProfile {
        code: "deu",
        unique_chars: &['ß'],
        shared_chars: &['ä', 'ö', 'ü'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ñ', 'ł', 'ą', 'ę', 'ě', 'ř'],
        signature_words: &[
            "nicht", "ich", "wir", "mich", "mir", "dich", "dir", "sich", "auch", "noch", "schon", "wenn", "hier", "jetzt",
            "immer", "etwas", "nichts", "warum", "wieder", "heute", "müssen", "können", "weiß", "bitte", "danke",
        ],
        common_words: &[
            "der", "die", "das", "und", "sie", "ist", "ein", "eine", "mit", "auf", "für", "aber", "was", "wie", "haben", "sind",
            "war", "hat", "nur", "oder", "von", "dem", "den",
        ],
    },
    LangLinguisticProfile {
        code: "fra",
        unique_chars: &['œ'],
        shared_chars: &['é', 'è', 'ê', 'à', 'ç', 'ù', 'î', 'ô', 'û'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ñ', 'ł', 'ě', 'ř'],
        signature_words: &[
            "c'est", "j'ai", "n'est", "qu'il", "qu'on", "d'un", "d'une", "dans", "avec", "nous", "vous", "mais", "tout", "tous",
            "plus", "faire", "quoi", "pourquoi", "comment", "jamais", "toujours", "merci", "vraiment", "suis", "êtes", "sont",
        ],
        common_words: &[
            "les", "des", "est", "pas", "que", "une", "pour", "qui", "sur", "mon", "ton", "son", "bien", "ici", "oui", "non",
            "elle", "ils", "aux",
        ],
    },
    LangLinguisticProfile {
        code: "spa",
        unique_chars: &['ñ', '¿', '¡'],
        shared_chars: &['á', 'é', 'í', 'ó', 'ú', 'ü'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ã', 'õ', 'ç', 'ł', 'ě', 'ř'],
        signature_words: &[
            "qué", "cómo", "aquí", "está", "están", "estoy", "vamos", "hacer", "tiene", "tengo", "pero", "porque", "cuando",
            "donde", "dónde", "nada", "algo", "todo", "todos", "gracias", "bueno", "ahora", "siempre", "nunca", "nosotros",
            "ustedes",
        ],
        common_words: &[
            "los", "las", "del", "por", "con", "una", "para", "son", "como", "más", "sus", "esta", "este", "esto", "bien", "muy",
            "ella", "ellos", "eso",
        ],
    },
    LangLinguisticProfile {
        code: "ita",
        unique_chars: &[],
        shared_chars: &['à', 'è', 'é', 'ì', 'ò', 'ù'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ñ', 'ã', 'õ', 'ł'],
        signature_words: &[
            "perché", "questo", "questa", "quello", "anche", "tutto", "tutti", "essere", "hanno", "siamo", "sono", "fatto",
            "sempre", "ancora", "adesso", "grazie", "prego", "molto", "niente", "qualcosa", "allora", "dobbiamo",
        ],
        common_words: &[
            "che", "non", "per", "una", "con", "del", "della", "più", "bene", "fare", "cosa", "dove", "quando", "mio", "tuo",
            "suo", "lei", "lui", "noi", "voi",
        ],
    },
    LangLinguisticProfile {
        code: "por",
        unique_chars: &['ã', 'õ'],
        shared_chars: &['ç', 'á', 'é', 'í', 'ó', 'ú', 'â', 'ê', 'ô', 'à'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ñ', 'ł'],
        signature_words: &[
            "não", "você", "vocês", "então", "também", "muito", "isso", "isto", "fazer", "tenho", "estão", "estou", "agora",
            "sempre", "nunca", "obrigado", "obrigada", "porque", "quando", "onde", "tudo", "nada",
        ],
        common_words: &["que", "uma", "com", "para", "por", "mais", "como", "mas", "foi", "está", "bem", "aqui"],
    },
    LangLinguisticProfile {
        code: "nld",
        unique_chars: &[],
        shared_chars: &['ë', 'ï'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ñ', 'ł', 'ą', 'ę'],
        signature_words: &[
            "niet", "geen", "hebt", "heeft", "hebben", "jij", "jouw", "wij", "jullie", "maar", "naar", "goed", "mijn", "zijn",
            "deze", "waarom", "wanneer", "altijd", "nooit", "iets", "niets", "moeten", "kunnen",
        ],
        common_words: &["het", "een", "van", "dat", "voor", "met", "wat", "heb", "ook", "nog", "wel", "hier"],
    },
    LangLinguisticProfile {
        code: "pol",
        unique_chars: &['ł', 'ą', 'ę', 'ń', 'ś', 'ź', 'ż'],
        shared_chars: &['ć', 'ó'],
        forbidden_chars: &['č', 'š', 'ž', 'đ', 'ě', 'ř', 'ä', 'ö', 'ü', 'ß'],
        signature_words: &[
            "się",
            "jest",
            "nie",
            "tylko",
            "już",
            "jeszcze",
            "bardzo",
            "tutaj",
            "gdzie",
            "kiedy",
            "dlaczego",
            "więc",
            "dobrze",
            "może",
            "będzie",
            "było",
            "mnie",
            "ciebie",
            "wszystko",
            "dziękuję",
            "proszę",
        ],
        common_words: &["jak", "ale", "czy", "tak", "jego", "jej", "mam", "masz", "jesteś", "jestem"],
    },
    LangLinguisticProfile {
        code: "ces",
        unique_chars: &['ě', 'ř', 'ů'],
        shared_chars: &['č', 'š', 'ž', 'á', 'é', 'í', 'ó', 'ú', 'ý', 'ť', 'ď', 'ň'],
        forbidden_chars: &['ć', 'đ', 'ł', 'ą', 'ę', 'ä', 'ö', 'ß'],
        signature_words: &[
            "jsem", "jsme", "jste", "jsou", "není", "když", "který", "která", "ještě", "tady", "proč", "nebo", "také", "velmi",
            "dobře", "můžu", "bude", "děkuji", "prosím", "něco", "nic", "všechno",
        ],
        common_words: &["jsi", "jak", "ale", "aby", "ano", "kde", "kdy", "mám", "máš"],
    },
    LangLinguisticProfile {
        code: "slk",
        unique_chars: &['ľ', 'ĺ', 'ŕ', 'ô'],
        shared_chars: &['ä', 'č', 'š', 'ž', 'á', 'é', 'í', 'ó', 'ú', 'ý', 'ť', 'ď', 'ň'],
        forbidden_chars: &['ć', 'đ', 'ě', 'ř', 'ů', 'ł', 'ą', 'ę', 'ß'],
        signature_words: &[
            "som", "sme", "ste", "nie", "keď", "ktorý", "ktorá", "ešte", "teraz", "prečo", "lebo", "veľmi", "dobre", "môžem",
            "nech", "ďakujem", "prosím", "niečo", "všetko",
        ],
        common_words: &["ako", "ale", "áno", "kde", "kedy", "bude", "mám"],
    },
    LangLinguisticProfile {
        code: "hun",
        unique_chars: &['ő', 'ű'],
        shared_chars: &['á', 'é', 'í', 'ó', 'ö', 'ú', 'ü'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ł'],
        signature_words: &[
            "hogy",
            "nem",
            "csak",
            "már",
            "mint",
            "vagy",
            "volt",
            "lesz",
            "igen",
            "nekem",
            "neked",
            "nagyon",
            "miért",
            "amikor",
            "minden",
            "semmi",
            "köszönöm",
            "kérem",
        ],
        common_words: &["egy", "van", "meg", "itt", "ott", "azt", "ezt", "még"],
    },
    LangLinguisticProfile {
        code: "ron",
        unique_chars: &['ă', 'ș', 'ț'],
        shared_chars: &['â', 'î'],
        forbidden_chars: &['č', 'ć', 'ž', 'š', 'đ', 'ß', 'ł', 'ä', 'ö', 'ü'],
        signature_words: &[
            "este",
            "sunt",
            "pentru",
            "care",
            "aici",
            "acolo",
            "foarte",
            "când",
            "unde",
            "ceva",
            "nimic",
            "acum",
            "doar",
            "mulțumesc",
            "trebuie",
            "poate",
        ],
        common_words: &["nu", "mai", "cum", "dar", "sau", "bine", "asta", "fost"],
    },
    LangLinguisticProfile {
        code: "tur",
        unique_chars: &['ğ', 'ı'],
        shared_chars: &['ş', 'ç', 'ö', 'ü'],
        forbidden_chars: &['č', 'ć', 'ž', 'đ', 'ß', 'ł'],
        signature_words: &[
            "için",
            "değil",
            "çok",
            "daha",
            "gibi",
            "nasıl",
            "neden",
            "burada",
            "şimdi",
            "evet",
            "hayır",
            "teşekkür",
            "lütfen",
            "biraz",
            "hiçbir",
            "herkes",
        ],
        common_words: &["bir", "bu", "ama", "var", "yok", "ben", "sen", "biz", "siz"],
    },
    LangLinguisticProfile {
        code: "swe",
        unique_chars: &[],
        shared_chars: &['å', 'ä', 'ö'],
        forbidden_chars: &['æ', 'ø', 'ß', 'č', 'ć', 'ž', 'š', 'đ'],
        signature_words: &[
            "och", "inte", "jag", "är", "för", "här", "när", "från", "mycket", "något", "inget", "alltid", "aldrig", "varför",
            "tack", "måste",
        ],
        common_words: &["att", "det", "som", "med", "har", "dig", "mig", "ett", "vad"],
    },
    LangLinguisticProfile {
        code: "dan",
        unique_chars: &[],
        shared_chars: &['æ', 'ø', 'å'],
        forbidden_chars: &['ä', 'ö', 'ß', 'č', 'ć', 'ž', 'š'],
        signature_words: &[
            "ikke", "jeg", "hvad", "hvorfor", "hvordan", "noget", "aldrig", "altid", "mange", "måske", "tak", "undskyld",
        ],
        common_words: &["og", "det", "at", "har", "til", "på", "med", "for", "som", "mig", "dig", "kan", "vil", "skal"],
    },
    LangLinguisticProfile {
        code: "nor",
        unique_chars: &[],
        shared_chars: &['æ', 'ø', 'å'],
        forbidden_chars: &['ä', 'ö', 'ß', 'č', 'ć', 'ž', 'š'],
        signature_words: &[
            "ikke", "jeg", "hva", "hvorfor", "hvordan", "noe", "aldri", "alltid", "kanskje", "takk", "unnskyld", "meg", "deg",
        ],
        common_words: &["og", "det", "på", "er", "til", "har", "med", "for", "her", "kan", "vil", "skal", "fra"],
    },
    LangLinguisticProfile {
        code: "fin",
        unique_chars: &[],
        shared_chars: &['ä', 'ö'],
        forbidden_chars: &['å', 'æ', 'ø', 'ß', 'č', 'ć', 'ž', 'š'],
        signature_words: &[
            "että", "ovat", "mutta", "niin", "vain", "kuin", "tämä", "mitä", "missä", "miksi", "minä", "sinä", "hän", "kyllä",
            "kiitos", "ei", "ole",
        ],
        common_words: &["on", "kun", "se", "nyt", "täällä", "kanssa"],
    },
];

const CYRILLIC_PROFILES: &[LangLinguisticProfile] = &[
    LangLinguisticProfile {
        code: "rus",
        unique_chars: &['ы', 'э', 'ъ', 'ё'],
        shared_chars: &[],
        forbidden_chars: &['і', 'ї', 'є', 'ґ', 'ђ', 'ћ', 'љ', 'њ', 'џ'],
        signature_words: &[
            "что",
            "это",
            "меня",
            "тебя",
            "здесь",
            "очень",
            "только",
            "сейчас",
            "когда",
            "почему",
            "ничего",
            "спасибо",
            "хорошо",
            "может",
            "будет",
            "было",
        ],
        common_words: &["как", "не", "так", "его", "она", "они", "мы", "вы", "ты", "да", "нет", "но", "все"],
    },
    LangLinguisticProfile {
        code: "ukr",
        unique_chars: &['ї', 'є', 'ґ'],
        shared_chars: &['і'],
        forbidden_chars: &['ы', 'э', 'ъ', 'ђ', 'ћ', 'љ', 'њ', 'џ'],
        signature_words: &[
            "що",
            "це",
            "він",
            "вона",
            "вони",
            "але",
            "дуже",
            "тут",
            "зараз",
            "чому",
            "нічого",
            "дякую",
            "добре",
            "може",
            "буде",
            "мені",
            "тобі",
        ],
        common_words: &["як", "не", "так", "ми", "ви", "ти", "та", "чи", "все"],
    },
    LangLinguisticProfile {
        code: "srp",
        unique_chars: &['ђ', 'ћ', 'љ', 'њ', 'џ'],
        shared_chars: &['ј'],
        forbidden_chars: &['ы', 'э', 'ъ', 'ё', 'і', 'ї', 'є', 'ґ', 'й', 'щ', 'ю', 'я'],
        signature_words: &[
            "шта",
            "што",
            "није",
            "нисам",
            "овде",
            "увек",
            "време",
            "зашто",
            "ништа",
            "нешто",
            "хвала",
            "молим",
            "добро",
            "можда",
            "нећу",
            "хоћеш",
        ],
        common_words: &["је", "су", "сам", "смо", "сте", "како", "али", "само", "тако", "све"],
    },
    LangLinguisticProfile {
        code: "bul",
        unique_chars: &[],
        shared_chars: &['ъ', 'щ', 'ю', 'я'],
        forbidden_chars: &['ы', 'э', 'ё', 'і', 'ї', 'є', 'ђ', 'ћ', 'љ', 'њ', 'џ'],
        signature_words: &[
            "какво",
            "защо",
            "тук",
            "много",
            "това",
            "нещо",
            "нищо",
            "съм",
            "сме",
            "сте",
            "няма",
            "добре",
            "благодаря",
            "може",
            "ще",
        ],
        common_words: &["е", "са", "не", "да", "на", "за", "от", "но", "ако", "как"],
    },
];

fn score_linguistic_profile(profile: &LangLinguisticProfile, texts: &[&str]) -> f32 {
    let mut score = 0.0_f32;

    for raw in texts {
        let lower = raw.to_lowercase();
        for ch in lower.chars() {
            if profile.unique_chars.contains(&ch) {
                score += 10.0;
            } else if profile.shared_chars.contains(&ch) {
                score += 4.5;
            } else if profile.forbidden_chars.contains(&ch) {
                score -= 7.0;
            }
        }

        for word in lower
            .split(|c: char| !c.is_alphabetic() && c != '\'' && c != '’')
            .map(|w| w.trim_matches(|c| c == '\'' || c == '’'))
            .filter(|w| !w.is_empty())
        {
            let norm = word.replace('’', "'");
            if profile.signature_words.contains(&norm.as_str()) {
                score += 14.0;
            } else if profile.common_words.contains(&norm.as_str()) {
                score += 4.5;
            }
        }
    }

    score
}

/// Selects up to `max_samples` Complete display sets distributed across `10%..85%` of the stream
/// so studio logos and end credits are avoided in favor of real spoken dialogue.
fn select_dialogue_tiff_samples(display_sets: &[PgsDisplaySet], max_samples: usize, cancel: &AtomicBool) -> Vec<Vec<u8>> {
    let complete: Vec<&PgsDisplaySet> = display_sets.iter().filter(|ds| ds.state() == PgsDisplaySetState::Complete).collect();

    if complete.is_empty() || max_samples == 0 {
        return Vec::new();
    }

    let n = complete.len();
    let indices: Vec<usize> = if n <= max_samples {
        if n > 2 {
            (1..n).collect()
        } else {
            (0..n).collect()
        }
    } else {
        let start = (n / 10).max(1);
        let end = ((n * 85) / 100).max(start + 1).min(n - 1);
        let span = end.saturating_sub(start);
        (0..max_samples).map(|k| start + (k * span) / (max_samples - 1).max(1)).collect()
    };

    let mut samples = Vec::with_capacity(indices.len());
    for idx in indices {
        if cancel.load(Ordering::Relaxed) {
            return Vec::new();
        }
        if let Ok(cursor) = get_tiff_stream(complete[idx]) {
            let bytes = cursor.into_inner();
            if !bytes.is_empty() {
                samples.push(bytes);
            }
        }
    }
    samples
}

/// Probes candidate installed languages directly from a `.sup` file path.
pub fn probe_language_from_file(path: &Path, candidate_languages: &[String]) -> Option<String> {
    probe_language_from_file_cancellable(path, candidate_languages, &AtomicBool::new(false))
}

/// Probes candidate installed languages directly from a `.sup` file path, aborting promptly when `cancel` is set.
pub fn probe_language_from_file_cancellable(path: &Path, candidate_languages: &[String], cancel: &AtomicBool) -> Option<String> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let path_str = path.to_str()?;
    let parser = pgs::PgsParser::parse(path_str).ok()?;
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    probe_language_from_display_sets_cancellable(parser.get_display_sets(), candidate_languages, cancel)
}

/// Probes candidate installed languages against representative dialogue cues in `display_sets`.
pub fn probe_language_from_display_sets(display_sets: &[PgsDisplaySet], candidate_languages: &[String]) -> Option<String> {
    probe_language_from_display_sets_cancellable(display_sets, candidate_languages, &AtomicBool::new(false))
}

/// Probes candidate installed languages against representative dialogue cues in `display_sets`, aborting when `cancel` is set.
pub fn probe_language_from_display_sets_cancellable(
    display_sets: &[PgsDisplaySet],
    candidate_languages: &[String],
    cancel: &AtomicBool,
) -> Option<String> {
    let samples = select_dialogue_tiff_samples(display_sets, 10, cancel);
    probe_language_from_tiff_samples_cancellable(&samples, candidate_languages, cancel)
}

/// Two-stage visual + linguistic language detector operating on encoded TIFF subtitle bitmaps:
/// 1. Identifies the genuine writing script (Latin, Cyrillic, Japanese, Chinese, Korean, etc.)
///    using Tesseract confidence (`mean_text_conf`) and anti-hallucination glyph checks.
/// 2. Disambiguates languages within that script (e.g. Croatian `hrv` vs English `eng`, German `deu`,
///    Serbian `srp_latn`, Slovenian `slv`, Czech `ces`, Polish `pol`) via diacritics, stopwords,
///    and finalist OCR confidence verification.
pub fn probe_language_from_tiff_samples(tiff_samples: &[Vec<u8>], candidate_languages: &[String]) -> Option<String> {
    probe_language_from_tiff_samples_cancellable(tiff_samples, candidate_languages, &AtomicBool::new(false))
}

/// Cancellable variant of [`probe_language_from_tiff_samples`].
pub fn probe_language_from_tiff_samples_cancellable(
    tiff_samples: &[Vec<u8>],
    candidate_languages: &[String],
    cancel: &AtomicBool,
) -> Option<String> {
    if tiff_samples.is_empty() || cancel.load(Ordering::Relaxed) {
        return None;
    }

    let candidates: Vec<String> = candidate_languages.iter().filter(|c| is_selectable_ocr_language(c)).cloned().collect();
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return Some(candidates[0].clone());
    }

    let quick_samples = &tiff_samples[..tiff_samples.len().min(5)];
    let (winning_family, anchor_fallback) = probe_anchor_script_family(&candidates, quick_samples, cancel)?;

    let family_candidates: Vec<String> =
        candidates.iter().filter(|c| language_script_family(c) == winning_family).cloned().collect();

    if family_candidates.len() == 1 {
        return Some(family_candidates[0].clone());
    }

    // Stage 2: Linguistic + Diacritic disambiguation inside the winning script family.
    match winning_family {
        ScriptFamily::Latin => {
            disambiguate_latin_candidates(&family_candidates, tiff_samples, anchor_fallback.as_deref(), cancel)
        }
        ScriptFamily::Cyrillic => disambiguate_cyrillic_candidates(&family_candidates, tiff_samples, quick_samples, cancel),
        _ => disambiguate_by_confidence(&family_candidates, quick_samples, cancel),
    }
}

fn probe_anchor_script_family(
    candidates: &[String],
    quick_samples: &[Vec<u8>],
    cancel: &AtomicBool,
) -> Option<(ScriptFamily, Option<String>)> {
    let has = |code: &str| candidates.iter().any(|c| c == code);
    let pick_first = |prefs: &[&str], family: ScriptFamily| -> Option<String> {
        for &p in prefs {
            if has(p) {
                return Some(p.to_string());
            }
        }
        candidates.iter().find(|c| language_script_family(c) == family).cloned()
    };

    let mut script_anchors: Vec<(ScriptFamily, String)> = Vec::new();
    for (family, prefs) in [
        (ScriptFamily::Latin, &["hrv", "ces", "deu", "fra", "spa", "eng"][..]),
        (ScriptFamily::Cyrillic, &["rus", "srp", "ukr", "bul"][..]),
        (ScriptFamily::Japanese, &["jpn"][..]),
        (ScriptFamily::Chinese, &["chi_sim", "chi_tra"][..]),
        (ScriptFamily::Korean, &["kor"][..]),
        (ScriptFamily::Greek, &["ell"][..]),
        (ScriptFamily::Arabic, &["ara", "fas"][..]),
        (ScriptFamily::Hebrew, &["heb"][..]),
        (ScriptFamily::Other, &["hin", "tha"][..]),
    ] {
        if let Some(anchor) = pick_first(prefs, family) {
            script_anchors.push((family, anchor));
        }
    }

    let mut script_results: Vec<(ScriptFamily, String, OcrSampleRun, f32)> = Vec::new();
    for (family, anchor) in script_anchors {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(run) = run_ocr_on_tiff_samples(&anchor, quick_samples, cancel) {
            let fit = evaluate_script_fit(family, &run);
            if fit > 0.0 {
                script_results.push((family, anchor, run, fit));
            }
        }
    }

    if script_results.is_empty() || cancel.load(Ordering::Relaxed) {
        return None;
    }

    // If Latin achieved high confidence (>= 70) with clean multi-letter words, suppress CJK/Other hallucinations.
    let latin_strong = script_results
        .iter()
        .find(|(f, _, _, _)| *f == ScriptFamily::Latin)
        .is_some_and(|(_, _, run, fit)| run.mean_conf >= 70.0 && *fit >= 45.0);

    if latin_strong {
        script_results.retain(|(f, _, _, _)| {
            !matches!(f, ScriptFamily::Japanese | ScriptFamily::Chinese | ScriptFamily::Korean | ScriptFamily::Other)
        });
    }

    script_results.sort_by(|a, b| b.3.total_cmp(&a.3));
    let winning_family = script_results[0].0;
    let anchor_fallback = script_results.first().map(|r| r.1.clone());
    Some((winning_family, anchor_fallback))
}

fn disambiguate_latin_candidates(
    family_candidates: &[String],
    tiff_samples: &[Vec<u8>],
    fallback_anchor: Option<&str>,
    cancel: &AtomicBool,
) -> Option<String> {
    let mut reader_langs: Vec<String> = Vec::new();
    for group in [
        &["hrv", "ces", "pol", "slk", "slv", "srp_latn", "bos"][..],
        &["deu", "fra", "spa", "ita", "por", "swe", "dan"][..],
        &["eng", "ron", "tur", "hun", "nld"][..],
    ] {
        if let Some(&picked) = group.iter().find(|&&g| family_candidates.iter().any(|c| c == g)) {
            if !reader_langs.iter().any(|r| r == picked) {
                reader_langs.push(picked.to_string());
            }
        }
    }
    if reader_langs.is_empty() {
        reader_langs.push(family_candidates[0].clone());
    }

    let mut reader_runs: Vec<(String, OcrSampleRun)> = Vec::new();
    for r_lang in &reader_langs {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(run) = run_ocr_on_tiff_samples(r_lang, tiff_samples, cancel) {
            reader_runs.push((r_lang.clone(), run));
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    if reader_runs.is_empty() {
        return fallback_anchor.map(|s| s.to_string());
    }

    let text_refs: Vec<&str> = reader_runs.iter().map(|(_, r)| r.text.as_str()).collect();
    let mut ranked: Vec<(String, f32)> = family_candidates
        .iter()
        .map(|cand| {
            let ling = LATIN_PROFILES.iter().find(|p| p.code == cand).map_or(0.0, |p| score_linguistic_profile(p, &text_refs));
            let tie_bias = match cand.as_str() {
                "hrv" | "eng" | "deu" | "fra" | "spa" | "ita" => 0.5,
                _ => 0.0,
            };
            (cand.clone(), ling + tie_bias)
        })
        .collect();

    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));

    let mut best_code: Option<String> = None;
    let mut best_total = f32::NEG_INFINITY;

    for (cand, pre_score) in ranked.into_iter().take(3) {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let run_opt = reader_runs
            .iter()
            .find(|(l, _)| l == &cand)
            .map(|(_, r)| r.clone())
            .or_else(|| run_ocr_on_tiff_samples(&cand, tiff_samples, cancel));

        if let Some(run) = run_opt {
            let own_ling = LATIN_PROFILES
                .iter()
                .find(|p| p.code == cand)
                .map_or(pre_score, |p| score_linguistic_profile(p, &[run.text.as_str()]));
            let combined_ling = own_ling.max(pre_score * 0.6);
            let total = combined_ling * 2.5 + run.mean_conf;
            if total > best_total {
                best_total = total;
                best_code = Some(cand);
            }
        }
    }

    (!cancel.load(Ordering::Relaxed)).then_some(best_code).flatten()
}

fn disambiguate_cyrillic_candidates(
    family_candidates: &[String],
    tiff_samples: &[Vec<u8>],
    quick_samples: &[Vec<u8>],
    cancel: &AtomicBool,
) -> Option<String> {
    let mut reader_runs: Vec<(String, OcrSampleRun)> = Vec::new();
    for group in [&["rus", "ukr"][..], &["srp", "bul"][..]] {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(&picked) = group.iter().find(|&&g| family_candidates.iter().any(|c| c == g)) {
            if let Some(run) = run_ocr_on_tiff_samples(picked, tiff_samples, cancel) {
                reader_runs.push((picked.to_string(), run));
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    if reader_runs.is_empty() {
        if let Some(run) = run_ocr_on_tiff_samples(&family_candidates[0], tiff_samples, cancel) {
            reader_runs.push((family_candidates[0].clone(), run));
        }
    }
    let text_refs: Vec<&str> = reader_runs.iter().map(|(_, r)| r.text.as_str()).collect();

    let mut best_code = None;
    let mut best_total = f32::NEG_INFINITY;
    for cand in family_candidates {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let ling = CYRILLIC_PROFILES.iter().find(|p| p.code == cand).map_or(0.0, |p| score_linguistic_profile(p, &text_refs));
        let conf = reader_runs
            .iter()
            .find(|(l, _)| l == cand)
            .map(|(_, r)| r.mean_conf)
            .or_else(|| run_ocr_on_tiff_samples(cand, quick_samples, cancel).map(|r| r.mean_conf))
            .unwrap_or(0.0);
        let total = ling * 2.5 + conf;
        if total > best_total {
            best_total = total;
            best_code = Some(cand.clone());
        }
    }
    (!cancel.load(Ordering::Relaxed)).then_some(best_code).flatten()
}

fn disambiguate_by_confidence(family_candidates: &[String], quick_samples: &[Vec<u8>], cancel: &AtomicBool) -> Option<String> {
    let mut best_code = None;
    let mut best_conf = -1.0_f32;
    for cand in family_candidates.iter().take(4) {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(run) = run_ocr_on_tiff_samples(cand, quick_samples, cancel) {
            if run.mean_conf > best_conf {
                best_conf = run.mean_conf;
                best_code = Some(cand.clone());
            }
        }
    }
    (!cancel.load(Ordering::Relaxed)).then_some(best_code).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_filename_tags_and_no_movie_title_false_positives() {
        assert_eq!(detect_language_from_filename("movie.eng"), Some("eng"));
        assert_eq!(detect_language_from_filename("Avatar.2009.1080p.deu"), Some("deu"));
        assert_eq!(detect_language_from_filename("Series.S01E02.German"), Some("deu"));
        assert_eq!(detect_language_from_filename("film_hrv"), Some("hrv"));
        assert_eq!(detect_language_from_filename("Film.2023.Croatian.SDH"), Some("hrv"));
        assert_eq!(detect_language_from_filename("Film.2023.hr.forced"), Some("hrv"));
        assert_eq!(detect_language_from_filename("Film_Hrvatski"), Some("hrv"));
        assert_eq!(detect_language_from_filename("show[Spanish]"), Some("spa"));
        assert_eq!(detect_language_from_filename("Anime.01.ja"), Some("jpn"));
        assert_eq!(detect_language_from_filename("Anime.01 [jpn]"), Some("jpn"));

        // Crucial anti-false-positive checks (Croatian "ja" = "I", "dan" = "day", English "It"/"No", Spanish "De"/"El")
        assert_eq!(detect_language_from_filename("Kralj i ja"), None);
        assert_eq!(detect_language_from_filename("Kralj_i_ja"), None);
        assert_eq!(detect_language_from_filename("Ja.i.moj.brat.2020"), None);
        assert_eq!(detect_language_from_filename("Dan.nezavisnosti.2016"), None);
        assert_eq!(detect_language_from_filename("No.Time.To.Die.2021"), None);
        assert_eq!(detect_language_from_filename("It.2017.1080p.BluRay"), None);
        assert_eq!(detect_language_from_filename("La.Casa.De.Papel.S01E01"), None);
        assert_eq!(detect_language_from_filename("El.Camino.2019"), None);
        assert_eq!(detect_language_from_filename("random_unknown_file"), None);
    }

    #[test]
    fn linguistic_scorer_identifies_croatian_over_all_other_languages() {
        let croatian_dialogue = [
            "Što radiš ovdje? Ne znam što se dogodilo.",
            "Moramo poći kući prije mraka, vrijeme istječe.",
            "Zašto mi nisi rekao da dolaziš večeras?",
        ];
        let cro_score = score_linguistic_profile(LATIN_PROFILES.iter().find(|p| p.code == "hrv").unwrap(), &croatian_dialogue);
        for other in LATIN_PROFILES.iter().filter(|p| p.code != "hrv") {
            let s = score_linguistic_profile(other, &croatian_dialogue);
            assert!(cro_score > s + 30.0, "Expected Croatian ('hrv': {cro_score}) to decisively beat '{}': {s}", other.code);
        }

        let english_dialogue =
            ["What are you doing here? I don't know what happened.", "We have to get out of this place before they come back."];
        let eng_score = score_linguistic_profile(LATIN_PROFILES.iter().find(|p| p.code == "eng").unwrap(), &english_dialogue);
        for other in LATIN_PROFILES.iter().filter(|p| p.code != "eng") {
            let s = score_linguistic_profile(other, &english_dialogue);
            assert!(eng_score > s + 30.0, "Expected English ('eng': {eng_score}) to decisively beat '{}': {s}", other.code);
        }

        let german_dialogue =
            ["Was machst du hier? Ich weiß nicht, was passiert ist.", "Aber wir müssen jetzt sofort nach Hause gehen."];
        let deu_score = score_linguistic_profile(LATIN_PROFILES.iter().find(|p| p.code == "deu").unwrap(), &german_dialogue);
        for other in LATIN_PROFILES.iter().filter(|p| p.code != "deu") {
            let s = score_linguistic_profile(other, &german_dialogue);
            assert!(deu_score > s + 30.0, "Expected German ('deu': {deu_score}) to decisively beat '{}': {s}", other.code);
        }
    }
}
