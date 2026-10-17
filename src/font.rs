//! Font loading: a small Latin face first, with CJK-capable faces in the chain
//! read only when a glyph actually needs them, so a Chinese answer still renders
//! without paying for the 19 MB CJK collection on the first frame.

#[cfg(test)]
use ab_glyph::GlyphId;
use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One coverage plane of a rasterized glyph: 8-bit coverage plus where it sits
/// relative to the canonical pen position (the pen with only its subpixel part
/// kept).
#[derive(Default, Clone)]
pub struct GlyphPlane {
    pub min_x: i32,
    pub min_y: i32,
    pub w: u32,
    pub h: u32,
    pub coverage: Vec<u8>,
}

/// One rasterized glyph: the advance to move the pen by, and its coverage
/// planes — one for grayscale antialiasing, three (left/middle/right subpixel
/// sample) for LCD subpixel antialiasing.
pub struct GlyphBitmap {
    pub advance: f32,
    pub planes: Vec<GlyphPlane>,
}

/// Glyph cache key: (face index, glyph id, subpixel x bucket, subpixel y bucket,
/// antialiasing mode).
pub type GlyphKey = (u8, u16, u8, u8, u8);

/// Pixel size of the panel's text. One constant, because there is no font
/// command-line syntax to negotiate with.
pub const SIZE: f32 = 13.0;

pub struct PanelFont {
    /// Chain faces, index 0 = primary. Beyond the primary a face is only read
    /// when a glyph actually needs it: the CJK face is 19 MB on this machine,
    /// and reading it before the first frame just to draw Latin text costs the
    /// startup latency the panel exists to avoid.
    faces: RefCell<Vec<FaceState>>,
    /// Paths for faces 1.., parallel to `faces`; the index stays stable so the
    /// glyph cache keys stay valid across lazy loads.
    chain: Vec<(PathBuf, u32)>,
    /// Pixel size (em height).
    pub size: f32,
    pub ascent_px: f32,
    pub descent_px: f32,
    pub line_h: f32,
    /// Row height incl. padding.
    pub row_h: u32,
    /// Rasterized glyphs reused across frames: without this every frame
    /// re-rasterizes every glyph, which dominates a full (64-row) frame
    /// (~8.4 ms measured in wlmenu). Discarded wholesale when the font is
    /// re-derived for a new scale, so a scale change cannot serve stale bitmaps.
    pub glyphs: RefCell<HashMap<GlyphKey, GlyphBitmap>>,
}

enum FaceState {
    Loaded(FontVec),
    /// Not read yet (`chain[i - 1]`).
    Unread,
    /// Read or parse failed: never retried, or every missing glyph would pay
    /// for the failing I/O again.
    Failed,
}

/// An in-flight font load: started before the Wayland handshake, joined by the
/// event loop when it first needs a frame.
pub(crate) type FontLoad = std::thread::JoinHandle<Result<PanelFont, String>>;

/// Load on a thread, so the read + parse overlap the handshake instead of
/// preceding it. Joining is the caller's business.
pub(crate) fn spawn_load(size: f32) -> FontLoad {
    std::thread::spawn(move || PanelFont::load(size))
}

/// Row height incl. padding, from the size alone: ab_glyph's scaled `height()`
/// is just the requested `PxScale`, so this is face-independent and the surface
/// can be committed at its final size before any font is loaded.
/// Comfortable row height ≈1.5× font size: glyphs breathe top/bottom (the
/// vertical pad is a quarter of the em on each side), and the highlighted band
/// is an easier target to hit (Fitts).
pub(crate) fn row_h_for(size: f32) -> u32 {
    size.ceil() as u32 + 2 * (size * 0.25).round() as u32
}

/// A face we expect to find, in the order we want it. `family` picks the face
/// inside a collection by name rather than hard-coding index 0: Noto's CJK TTC
/// holds JP/KR/SC/TC faces of the same file, and index 0 is Japanese, which
/// draws a handful of ideographs in their Japanese form.
struct Candidate {
    path: &'static str,
    family: &'static str,
    /// CJK-capable: at most one of these is kept, and one is required.
    cjk: bool,
}

const CANDIDATES: &[Candidate] = &[
    Candidate {
        path: "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        family: "",
        cjk: false,
    },
    Candidate {
        path: "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
        family: "",
        cjk: false,
    },
    Candidate {
        path: "/usr/share/fonts/TTF/DejaVuSans.ttf",
        family: "",
        cjk: false,
    },
    Candidate {
        path: "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        family: "",
        cjk: false,
    },
    Candidate {
        path: "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        family: "",
        cjk: false,
    },
    Candidate {
        path: "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        family: "Noto Sans CJK SC",
        cjk: true,
    },
    Candidate {
        path: "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        family: "Noto Sans CJK SC",
        cjk: true,
    },
];

impl PanelFont {
    pub fn load(size: f32) -> Result<PanelFont, String> {
        let chain = chain();
        let (path, index) = chain
            .first()
            .cloned()
            .ok_or("no usable system font found")?;
        let bytes =
            std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let font = FontVec::try_from_vec_and_index(bytes, index)
            .map_err(|e| format!("invalid font {}: {e}", path.display()))?;
        let rest: Vec<(PathBuf, u32)> = chain.into_iter().skip(1).collect();
        Ok(Self::build(font, rest, size))
    }

    fn build(font: FontVec, chain: Vec<(PathBuf, u32)>, size: f32) -> PanelFont {
        let scaled = font.as_scaled(PxScale::from(size));
        let ascent_px = scaled.ascent();
        let descent_px = scaled.descent();
        let line_h = scaled.height(); // == size by definition of PxScale
        let row_h = row_h_for(size);
        let faces = std::iter::once(FaceState::Loaded(font))
            .chain(chain.iter().map(|_| FaceState::Unread))
            .collect();
        PanelFont {
            faces: RefCell::new(faces),
            chain,
            size,
            ascent_px,
            descent_px,
            line_h,
            row_h,
            glyphs: RefCell::new(HashMap::new()),
        }
    }

    /// Faces in the chain, primary included.
    pub fn face_count(&self) -> usize {
        self.faces.borrow().len()
    }

    /// Chain face `i` (0 = primary, always loaded), read on first use. `None`
    /// when that face cannot be read or parsed, so the chain skips it.
    pub fn face(&self, i: usize) -> Option<Ref<'_, FontVec>> {
        let unread = matches!(self.faces.borrow().get(i), Some(FaceState::Unread));
        if unread {
            let read = i
                .checked_sub(1)
                .and_then(|k| self.chain.get(k))
                .and_then(|(path, index)| read_face(path, *index));
            if let Some(slot) = self.faces.borrow_mut().get_mut(i) {
                *slot = match read {
                    Some(font) => FaceState::Loaded(font),
                    None => FaceState::Failed,
                };
            }
        }
        Ref::filter_map(self.faces.borrow(), |faces| match faces.get(i) {
            Some(FaceState::Loaded(font)) => Some(font),
            _ => None,
        })
        .ok()
    }

    /// Chain index + glyph id for `ch`, reading faces on demand.
    #[cfg(test)]
    fn glyph_for(&self, ch: char) -> Option<(usize, GlyphId)> {
        for i in 0..self.face_count() {
            let Some(face) = self.face(i) else {
                continue;
            };
            let gid = face.as_scaled(PxScale::from(self.size)).glyph_id(ch);
            if gid != GlyphId(0) {
                return Some((i, gid));
            }
        }
        None
    }

    /// True if any face in the chain (primary first) has a glyph for `ch`.
    #[cfg(test)]
    pub fn has_glyph(&self, ch: char) -> bool {
        self.glyph_for(ch).is_some()
    }

    /// Faces not read yet (test-only): startup must read nothing but the
    /// primary.
    #[cfg(test)]
    fn unread_faces(&self) -> usize {
        self.faces
            .borrow()
            .iter()
            .filter(|f| matches!(f, FaceState::Unread))
            .count()
    }
}

/// The chain: the candidate paths that exist, plus a CJK face from the font
/// database if none of the candidates is one.
///
/// Candidate paths are stat'ed first because the database scan below parses
/// every installed font header (~655 files here) and the first frame waits for
/// the primary face. The scan runs only on a machine whose fonts live somewhere
/// this list does not know.
fn chain() -> Vec<(PathBuf, u32)> {
    let mut out: Vec<(PathBuf, u32)> = Vec::new();
    for c in CANDIDATES {
        let path = PathBuf::from(c.path);
        if !path.exists() {
            continue;
        }
        if c.cjk && out.iter().any(|(p, _)| is_cjk_path(p)) {
            continue; // one big collection is enough
        }
        out.push((
            path,
            index_for_family(Path::new(c.path), c.family).unwrap_or(0),
        ));
    }
    if out.is_empty() {
        return system_chain();
    }
    if !out.iter().any(|(p, _)| is_cjk_path(p))
        && let Some(cjk) = system_chain().into_iter().find(|(p, _)| is_cjk_path(p))
    {
        out.push(cjk);
    }
    out
}

/// Whether a chain entry came from a CJK candidate. Kept by path rather than by
/// file size, so the check cannot misfire on some other large collection.
fn is_cjk_path(path: &Path) -> bool {
    CANDIDATES
        .iter()
        .any(|c| c.cjk && Path::new(c.path) == path)
}

/// Face index inside `path` whose family is `family`; `None` for a single-face
/// file or when the family is absent, and the caller then keeps index 0.
fn index_for_family(path: &Path, family: &str) -> Option<u32> {
    if family.is_empty() {
        return None;
    }
    let mut db = fontdb::Database::new();
    db.load_font_file(path).ok()?;
    db.faces()
        .find(|f| f.families.iter().any(|(name, _)| name.as_str() == family))
        .map(|f| f.index)
}

/// CJK-preferring ordered list from the font database (primary candidate first),
/// including the Nerd Font PUA face, used when the known paths do not exist.
fn system_chain() -> Vec<(PathBuf, u32)> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    // CJK list: mutual alternates, not complements. One face covers all CJK +
    // kana glyphs, and SC/TC/JP are usually faces of the same ~20MB TTC — so
    // take the first hit and stop.
    const CJK: &[&str] = &[
        "Noto Sans CJK SC",
        "Noto Sans CJK TC",
        "Noto Sans CJK JP",
        "WenQuanYi Micro Hei",
        "Source Han Sans SC",
    ];
    const SANS: &[&str] = &["Noto Sans", "DejaVu Sans", "Liberation Sans"];
    let mut out = Vec::new();
    for fam in SANS {
        if let Some(found) = query_path(&db, fam) {
            out.push(found);
        }
    }
    for fam in CJK {
        if let Some(found) = query_path(&db, fam) {
            out.push(found);
            break;
        }
    }
    // Nerd Font PUA icons (patched families / "Symbols Nerd Font"): the Latin
    // and CJK faces carry few of them, so a face whose family mentions
    // "Nerd Font" joins the chain to resolve them.
    for face in db.faces() {
        if face
            .families
            .iter()
            .any(|f| f.0.to_ascii_lowercase().contains("nerd font"))
            && let Some(found) = face_path(&db, face.id)
        {
            out.push(found);
            break;
        }
    }
    if out.is_empty() {
        // Give up on family preference: just take the first face so the panel
        // still renders.
        if let Some(face) = db.faces().next()
            && let Some(found) = face_path(&db, face.id)
        {
            out.push(found);
        }
    }
    out
}

fn query_path(db: &fontdb::Database, family: &str) -> Option<(PathBuf, u32)> {
    let id = db.query(&fontdb::Query {
        families: &[fontdb::Family::Name(family)],
        ..Default::default()
    })?;
    face_path(db, id)
}

fn face_path(db: &fontdb::Database, id: fontdb::ID) -> Option<(PathBuf, u32)> {
    let face = db.face(id)?;
    let path = match &face.source {
        fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => p.clone(),
        fontdb::Source::Binary(_) => return None,
    };
    Some((path, face.index))
}

/// Read + parse one chain face. Best-effort: a font that cannot be loaded is
/// skipped by the chain.
fn read_face(path: &Path, index: u32) -> Option<FontVec> {
    let bytes = std::fs::read(path).ok()?;
    FontVec::try_from_vec_and_index(bytes, index).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cjk_face_is_not_read_before_a_cjk_glyph() {
        // Startup must read the primary only: the CJK collection is 19 MB here
        // and reading it to draw Latin text would cost the first frame.
        let m = PanelFont::load(SIZE).expect("system font available");
        let fallbacks = m.face_count() - 1;
        if fallbacks == 0 {
            return; // single-font system: nothing to be lazy about
        }
        assert_eq!(m.unread_faces(), fallbacks, "only the primary was read");
        assert!(m.has_glyph('中'), "CJK must resolve on demand");
        assert!(
            m.unread_faces() < fallbacks,
            "the CJK lookup must have read at least one fallback face"
        );
    }

    #[test]
    fn chain_takes_one_cjk_face_only() {
        // SC/TC/JP/文泉驿/思源 are alternates, not complements (usually faces of
        // the same ~20MB TTC): the chain must never hold more than one big face.
        let chain = chain();
        let big = chain
            .iter()
            .filter(|(p, _)| {
                std::fs::metadata(p)
                    .map(|m| m.len() > 8_000_000)
                    .unwrap_or(false)
            })
            .count();
        assert!(big <= 1, "at most one large (CJK) face in chain: {chain:?}");
    }

    #[test]
    fn the_cjk_collection_face_is_picked_by_name_not_index_zero() {
        let Some(c) = CANDIDATES
            .iter()
            .find(|c| c.cjk && Path::new(c.path).exists())
        else {
            return; // no CJK collection installed
        };
        let index =
            index_for_family(Path::new(c.path), c.family).expect("named face in collection");
        let bytes = std::fs::read(c.path).unwrap();
        let face = FontVec::try_from_vec_and_index(bytes, index).expect("face parses");
        let scaled = face.as_scaled(PxScale::from(SIZE));
        assert_ne!(scaled.glyph_id('中'), GlyphId(0), "the SC face covers CJK");
    }

    #[test]
    fn provisional_row_height_matches_the_loaded_font() {
        // The surface commits at `row_h_for(size)` before the font exists, so
        // the two must agree for every size the panel can ask for.
        for size in [13.0, 16.0, 32.0, 20.0 * 96.0 / 72.0] {
            let m = PanelFont::load(size).expect("system font available");
            assert_eq!(m.row_h, row_h_for(size), "size {size}");
        }
    }

    #[test]
    fn primary_face_covers_latin() {
        let m = PanelFont::load(SIZE).expect("system font available");
        assert!(m.has_glyph('a') && m.has_glyph('1'));
    }

    #[test]
    fn fallback_chain_covers_cjk_for_latin_primary() {
        // Noto Sans carries no CJK glyphs; the chain must resolve them.
        let m = PanelFont::load(SIZE).expect("system font available");
        assert!(m.has_glyph('a'));
        assert!(
            m.has_glyph('中'),
            "CJK must resolve via the fallback chain when the primary is latin-only"
        );
    }
}
