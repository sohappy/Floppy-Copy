//! Format detection against the real Greaseweazle host tools, when they are installed (`gw` on
//! the PATH, run by a Python that has its compiled routines). Skipped otherwise.

use hgf_core::detect::{self, Line};
use std::process::Command;

fn gw_python() -> Option<std::path::PathBuf> {
    let py = detect::gw_python("gw")?;
    let ok =
        Command::new(&py).args(["-c", "import greaseweazle.optimised.optimised"]).status().ok()?.success();
    ok.then_some(py)
}

#[test]
fn detects_an_amiga_and_a_pc_disk() {
    let Some(py) = gw_python() else {
        eprintln!("skipped: no gw with its Python");
        return;
    };
    let dir = std::env::temp_dir().join(format!("fc-detect-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("detect.py");
    std::fs::write(&script, detect::SCRIPT).unwrap();
    // Sector images turned into flux of cylinders 0-2 by gw itself.
    let mut adf = vec![0x5Au8; 901_120];
    adf[..4].copy_from_slice(b"DOS\0");
    let mut img = vec![0xA5u8; 737_280];
    img[0] = 0xEB;
    img[3..11].copy_from_slice(b"MSDOS5.0");
    img[11..13].copy_from_slice(&512u16.to_le_bytes());
    img[19..21].copy_from_slice(&1440u16.to_le_bytes());
    img[24..26].copy_from_slice(&9u16.to_le_bytes());
    img[26..28].copy_from_slice(&2u16.to_le_bytes());
    for (name, data, format, expect) in
        [("a.adf", adf, "amiga.amigados", "amiga.amigados"), ("p.img", img, "ibm.720", "ibm.720")]
    {
        let src = dir.join(name);
        std::fs::write(&src, data).unwrap();
        let flux = dir.join(format!("{name}.scp"));
        let st = Command::new("gw")
            .args(["convert", "--format", format, "--tracks", "c=0-2"])
            .arg(&src)
            .arg(&flux)
            .output()
            .unwrap();
        assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stdout));
        let out = Command::new(&py).arg(&script).arg(&flux).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let cands: Vec<_> = text
            .lines()
            .filter_map(|l| match detect::parse_line(l) {
                Line::Result(c) => Some(c),
                _ => None,
            })
            .collect();
        assert!(cands.len() > 100, "{text}");
        let d = detect::rank(&cands);
        assert_eq!(d.best().map(|c| c.format.as_str()), Some(expect), "{}", d.summary());
        assert!(!d.double_step);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
