//! Font loading: a small Latin face first, with CJK-capable faces in the chain
//! read only when a glyph actually needs them, so a Chinese answer still renders
//! without paying for the 19 MB CJK collection on the first frame.

use skrifa::prelude::*;
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

/// What [`PanelFont::glyph`] answers with: chain face index, glyph id, and the
/// advance in pixels.
pub type GlyphLookup = (usize, GlyphId, f32);

/// Pixel size of the panel's text when the configuration does not say otherwise:
/// `wlmenu`'s bar size, so the two read as one product. 15 pt at 96 dpi;
/// `row_h_for` keeps the 1.5× line height and `PAD` stays 0.8 em, so the
/// proportions follow from it.
pub const SIZE: f32 = 20.0;

/// The smallest and largest configured size, in logical pixels. Below `8` the
/// panel is unreadable; above `72` a row is taller than most panels have room for.
pub const SIZE_RANGE: std::ops::RangeInclusive<u32> = 8..=72;

/// What the configuration said about the font, resolved once at startup and then
/// changeable from the panel's prefix. The panel is a single font at a single size for
/// the whole run — a size change reads the face again and swaps it in — so this is a
/// process-wide setting rather than a parameter threaded through every sizing function.
static CONFIGURED_SIZE: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(f32::to_bits(SIZE));
static CONFIGURED_FAMILY: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Take the configured font. Called once, before the first frame.
pub fn configure(size: f32, family: Option<String>) {
    resize(size);
    if let Some(family) = family {
        let _ = CONFIGURED_FAMILY.set(family);
    }
}

/// Change the text size for the rest of the run: the panel's sizes are derived from it,
/// so the caller re-reads the face and re-lays the transcript out.
pub fn resize(size: f32) {
    CONFIGURED_SIZE.store(size.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

/// The panel's text size: the configured one, or [`SIZE`].
pub fn size() -> f32 {
    f32::from_bits(CONFIGURED_SIZE.load(std::sync::atomic::Ordering::Relaxed))
}

/// The configured family, if any. A name nothing provides is not an error: it is
/// put at the head of the chain and the chain answers for it when it cannot.
pub fn family() -> Option<&'static str> {
    CONFIGURED_FAMILY.get().map(String::as_str)
}

/// Cap on the character lookup memo. Its keys come from the provider's text, so
/// a long session of answers full of distinct characters must not grow it with
/// the session; past the cap it resets, which costs one re-lookup per character.
const LOOKUP_CACHE_CAP: usize = 8192;

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
    /// `char` -> chain face index, glyph id and pixel advance, for this size.
    /// Neither can change while the font lives, so the memo cannot go stale;
    /// past `LOOKUP_CACHE_CAP` it is dropped wholesale instead of growing.
    lookups: RefCell<HashMap<char, Option<GlyphLookup>>>,
    /// The code face: a monospace chain at the same size, so a fence's columns line
    /// up. Built on the first code glyph and never before: reading it eagerly would
    /// put a font file read in front of every start, and a database scan in front of
    /// every font a test builds.
    mono: RefCell<CodeFace>,
}

/// The code face's state, so a failed build is not retried on every code row.
enum CodeFace {
    Unbuilt,
    Built(Box<PanelFont>),
    Failed,
}

enum FaceState {
    Loaded(Face),
    /// Not read yet (`chain[i - 1]`).
    Unread,
    /// Read or parse failed: never retried, or every missing glyph would pay
    /// for the failing I/O again.
    Failed,
}

/// One chain face. The bytes live here because a skrifa `FontRef` borrows them
/// (`ab_glyph`'s `FontVec` owned them instead), so a face hands one out for as
/// long as the borrow held by [`FaceRef`] lasts.
struct Face {
    bytes: Vec<u8>,
    index: u32,
    /// Unscaled ascender and descender. Kept because `ab_glyph` scaled every
    /// face by `size / (ascender - descender)` rather than by units-per-em, and
    /// every advance, baseline and outline follows from that one number.
    ascender: f32,
    descender: f32,
}

impl Face {
    fn new(bytes: Vec<u8>, index: u32) -> Result<Face, String> {
        let font = FontRef::from_index(&bytes, index).map_err(|e| e.to_string())?;
        let metrics = font.metrics(Size::unscaled(), LocationRef::default());
        Ok(Face {
            bytes,
            index,
            ascender: metrics.ascent,
            descender: metrics.descent,
        })
    }

    fn font(&self) -> Option<FontRef<'_>> {
        FontRef::from_index(&self.bytes, self.index).ok()
    }
}

/// A chain face, borrowed from the cell that holds it: the borrow travels with
/// the face because any `FontRef` built from it borrows the same bytes.
pub struct FaceRef<'a> {
    face: Ref<'a, Face>,
}

impl FaceRef<'_> {
    /// skrifa's view of this face. Cheap, but it re-reads the table directory,
    /// so a caller holds one for a whole run of glyphs.
    pub fn font(&self) -> Option<FontRef<'_>> {
        self.face.font()
    }

    /// Pixels per font unit at `size`.
    pub fn px_per_unit(&self, size: f32) -> f32 {
        size / (self.face.ascender - self.face.descender)
    }

    /// Glyph id and pixel advance for `c`, or `None` when this face has no
    /// glyph for it. Glyph 0 counts as missing, as it did under `ab_glyph`.
    pub fn glyph(&self, font: &FontRef<'_>, size: f32, c: char) -> Option<(GlyphId, f32)> {
        let gid = font.charmap().map(c)?;
        if gid.to_u32() == 0 {
            return None;
        }
        let advance = font
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(gid)
            .unwrap_or_default();
        Some((gid, advance * self.px_per_unit(size)))
    }
}

/// An in-flight font load: started before the Wayland handshake, joined by the
/// event loop when it first needs a frame.
pub(crate) type FontLoad = std::thread::JoinHandle<Result<PanelFont, String>>;

/// Load on a thread, so the read + parse overlap the handshake instead of
/// preceding it. Joining is the caller's business.
pub(crate) fn spawn_load(size: f32) -> FontLoad {
    let family = family().map(str::to_string);
    std::thread::spawn(move || PanelFont::load_with(size, family.as_deref()))
}

/// Row height incl. padding, from the size alone: a line is the requested size
/// tall, not the face's, so this is face-independent and the surface can be
/// committed at its final size before any font is loaded.
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
    /// The configured font, or the built-in chain: what every caller but a test
    /// wants.
    pub fn load(size: f32) -> Result<PanelFont, String> {
        Self::load_with(size, family())
    }

    /// `family` is put at the head of the chain; a family nothing provides is
    /// skipped, so a typo in the configuration cannot stop the panel starting.
    pub fn load_with(size: f32, family: Option<&str>) -> Result<PanelFont, String> {
        let chain = chain(family);
        let (path, index) = chain
            .first()
            .cloned()
            .ok_or("no usable system font found")?;
        let bytes =
            std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let face =
            Face::new(bytes, index).map_err(|e| format!("invalid font {}: {e}", path.display()))?;
        let rest: Vec<(PathBuf, u32)> = chain.into_iter().skip(1).collect();
        Ok(Self::build(face, rest, size))
    }

    /// The monospace chain, built the same way and failing softly: a system with no
    /// monospace face draws code in the text face.
    fn load_mono(size: f32) -> Option<PanelFont> {
        let chain = mono_chain();
        let (path, index) = chain.first().cloned()?;
        let bytes = std::fs::read(&path).ok()?;
        let face = Face::new(bytes, index).ok()?;
        let rest: Vec<(PathBuf, u32)> = chain.into_iter().skip(1).collect();
        Some(Self::build(face, rest, size))
    }

    /// Whether the code face has been read (test-only): a plain answer must not.
    #[cfg(test)]
    pub fn code_face_unbuilt(&self) -> bool {
        matches!(*self.mono.borrow(), CodeFace::Unbuilt)
    }

    /// The code face, built on first use. `None` when the system has no monospace
    /// face: the code is then drawn in the text face rather than not at all.
    pub fn mono(&self) -> Option<Ref<'_, PanelFont>> {
        if matches!(*self.mono.borrow(), CodeFace::Unbuilt) {
            let built = Self::load_mono(self.size);
            *self.mono.borrow_mut() = match built {
                Some(font) => CodeFace::Built(Box::new(font)),
                None => CodeFace::Failed,
            };
        }
        Ref::filter_map(self.mono.borrow(), |face| match face {
            CodeFace::Built(font) => Some(&**font),
            _ => None,
        })
        .ok()
    }

    fn build(face: Face, chain: Vec<(PathBuf, u32)>, size: f32) -> PanelFont {
        let px_per_unit = face.ascender - face.descender;
        let ascent_px = face.ascender * size / px_per_unit;
        let descent_px = face.descender * size / px_per_unit;
        let line_h = size; // a line is the requested size: nothing measures a font's own height
        let row_h = row_h_for(size);
        let faces = std::iter::once(FaceState::Loaded(face))
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
            lookups: RefCell::new(HashMap::new()),
            mono: RefCell::new(CodeFace::Unbuilt),
        }
    }

    /// Faces in the chain, primary included.
    pub fn face_count(&self) -> usize {
        self.faces.borrow().len()
    }

    /// Chain face `i` (0 = primary, always loaded), read on first use. `None`
    /// when that face cannot be read or parsed, so the chain skips it.
    pub fn face(&self, i: usize) -> Option<FaceRef<'_>> {
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
            Some(FaceState::Loaded(face)) => Some(face),
            _ => None,
        })
        .ok()
        .map(|face| FaceRef { face })
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

    /// Chain face index, glyph id and pixel advance for `ch`, or `None` when no
    /// face in the chain has it.
    ///
    /// The chain is asked once per character and the answer kept: the mapping
    /// and the advance cannot change while the font lives (a new size is a new
    /// `PanelFont`), and without the memo every frame re-walked the cmap of
    /// every face up to the CJK one.
    pub fn glyph(&self, ch: char) -> Option<GlyphLookup> {
        if let Some(hit) = self.lookups.borrow().get(&ch) {
            return *hit;
        }
        let found = (0..self.face_count()).find_map(|i| {
            let face = self.face(i)?;
            let font = face.font()?;
            let (gid, advance) = face.glyph(&font, self.size, ch)?;
            Some((i, gid, advance))
        });
        if self.lookups.borrow().len() >= LOOKUP_CACHE_CAP {
            self.lookups.borrow_mut().clear();
        }
        self.lookups.borrow_mut().insert(ch, found);
        found
    }

    /// True if any face in the chain (primary first) has a glyph for `ch`.
    #[cfg(test)]
    pub fn has_glyph(&self, ch: char) -> bool {
        self.glyph(ch).is_some()
    }
}

/// The chain: the candidate paths that exist, plus a CJK face from the font
/// database if none of the candidates is one.
///
/// Candidate paths are stat'ed first because the database scan below parses
/// every installed font header (~655 files here) and the first frame waits for
/// the primary face. The scan runs only on a machine whose fonts live somewhere
/// this list does not know.
fn chain(family: Option<&str>) -> Vec<(PathBuf, u32)> {
    let mut out: Vec<(PathBuf, u32)> = Vec::new();
    // The configured family first, whatever else the known paths hold: it is what
    // the user asked for, and the rest of the chain is the fallback for the
    // scripts it cannot draw.
    if let Some(family) = family.filter(|f| !f.trim().is_empty()) {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        if let Some(found) = query_path(&db, family) {
            out.push(found);
        }
    }
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
    // A configured family is a preference for the text, not a replacement for the
    // CJK fallback: one face covers those glyphs, and without it an answer in
    // Chinese would render as blanks.
    if !out.iter().any(|(p, _)| is_cjk_path(p))
        && let Some(cjk) = system_chain().into_iter().find(|(p, _)| is_cjk_path(p))
    {
        out.push(cjk);
    }
    out
}

/// Monospace candidates, in preference order. Stat'ed instead of queried for the
/// same reason as [`CANDIDATES`]: a font database scan reads every installed font's
/// header, and this runs when the first code block is drawn.
const MONO_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf",
    "/usr/share/fonts/noto-cjk/NotoSansMonoCJK-Regular.ttc",
];

/// The code chain: a monospace face in front of the same fallbacks the text chain
/// uses, so a comment in Chinese inside a fence still draws.
fn mono_chain() -> Vec<(PathBuf, u32)> {
    let mut out = chain(None);
    let head = MONO_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .map(|path| {
            let index = index_for_family(&path, "").unwrap_or(0);
            (path, index)
        })
        .or_else(mono_path);
    if let Some(head) = head {
        out.insert(0, head);
    }
    out
}

/// The font database's idea of monospace, or `None` when it has none.
fn mono_path() -> Option<(PathBuf, u32)> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let id = db.query(&fontdb::Query {
        families: &[fontdb::Family::Monospace],
        ..Default::default()
    })?;
    face_path(&db, id)
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
fn read_face(path: &Path, index: u32) -> Option<Face> {
    let bytes = std::fs::read(path).ok()?;
    Face::new(bytes, index).ok()
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

    /// The configured family goes first, and a name nothing provides is skipped
    /// rather than fatal — a typo in the file must not stop the panel starting.
    #[test]
    fn a_configured_family_leads_the_chain_and_a_bad_one_is_skipped() {
        let plain = chain(None);
        let first = plain.first().cloned().expect("a system font");
        let family = family_of(&first.0, first.1).expect("a family name");
        let named = chain(Some(&family));
        assert_eq!(
            named.first(),
            Some(&first),
            "the family it names is already first; asking for it changes nothing"
        );
        let nonsense = chain(Some("No Such Family 1234"));
        assert_eq!(
            nonsense.first(),
            plain.first(),
            "an unknown family falls back to the built-in chain"
        );
        assert!(!nonsense.is_empty(), "and the panel still has a font");
    }

    /// The family of the face at `index` in `path`, for a test that has to name one.
    fn family_of(path: &Path, index: u32) -> Option<String> {
        let mut db = fontdb::Database::new();
        db.load_font_file(path).ok()?;
        db.faces()
            .find(|f| f.index == index)
            .and_then(|f| f.families.first().map(|(name, _)| name.clone()))
    }

    #[test]
    fn chain_takes_one_cjk_face_only() {
        // SC/TC/JP/文泉驿/思源 are alternates, not complements (usually faces of
        // the same ~20MB TTC): the chain must never hold more than one big face.
        let chain = chain(None);
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
        let face = Face::new(bytes, index).expect("face parses");
        let font = face.font().expect("face parses");
        let gid = font.charmap().map('中').expect("CJK mapping");
        assert_ne!(gid.to_u32(), 0, "the SC face covers CJK");
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

    #[test]
    fn the_glyph_memo_stays_bounded() {
        // The keys come from the provider's text, so the memo must not grow with
        // the session: past the cap it resets instead.
        let m = PanelFont::load(SIZE).expect("system font available");
        for i in 0..(LOOKUP_CACHE_CAP * 2) {
            let ch = char::from_u32(0x4e00 + i as u32).expect("a BMP scalar");
            let _ = m.glyph(ch);
        }
        let held = m.lookups.borrow().len();
        assert!(held <= LOOKUP_CACHE_CAP, "the memo is capped, held {held}");
    }

    #[test]
    fn the_glyph_memo_answers_what_the_chain_does() {
        // The chain is walked once per character and the answer kept for the
        // life of the font. A font that re-walks the chain must agree, or the
        // memo would be hiding a face that should have answered first.
        let m = PanelFont::load(SIZE).expect("system font available");
        let fresh = PanelFont::load(SIZE).expect("system font available");
        for ch in ['a', 'W', '中', '火', '😀'] {
            let hit = m.glyph(ch);
            assert_eq!(hit, fresh.glyph(ch), "{ch:?}: the memo changed the answer");
            assert_eq!(hit, m.glyph(ch), "{ch:?}: asking twice is asking once");
            if let Some((_, gid, advance)) = hit {
                assert!(gid.to_u32() != 0, "{ch:?}: glyph 0 is not a glyph");
                assert!(advance > 0.0, "{ch:?}: a real glyph has a real advance");
            }
        }
    }
}
