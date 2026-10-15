//! Compiles the vendored upstream libspeex 1.2.1 decoder sources and Studio's C shim with `cc`.
//! No bindgen: the Rust side declares the shim's eight functions by hand (`src/ffi.rs`).
//!
//! Recipe (recorded in `README.md`): the upstream `libspeex_la_SOURCES` list without the optional
//! Vorbis psychoacoustic / FFT sources (not enabled by upstream's default configure); defines
//! `FLOATING_POINT` (upstream's default), `USE_ALLOCA` (stack arrays through alloca, as upstream's
//! Win32 config without SSE), `EXPORT=` and `OS_SUPPORT_CUSTOM` (the shim's fatal / warning hooks);
//! `inline=__inline` for MSVC's C compiler. No SSE / ARM / Blackfin paths, no upstream edits.

fn main() {
    const SOURCES: &[&str] = &[
        "cb_search.c", "exc_10_32_table.c", "exc_8_128_table.c", "filters.c", "gain_table.c", "hexc_table.c",
        "high_lsp_tables.c", "lsp.c", "ltp.c", "speex.c", "stereo.c", "vbr.c", "vq.c", "bits.c", "exc_10_16_table.c",
        "exc_20_32_table.c", "exc_5_256_table.c", "exc_5_64_table.c", "gain_table_lbr.c", "hexc_10_32_table.c",
        "lpc.c", "lsp_tables_nb.c", "modes.c", "modes_wb.c", "nb_celp.c", "quant_lsp.c", "sb_celp.c",
        "speex_callbacks.c", "speex_header.c", "window.c",
    ];
    let vendor = "vendor/speex-1.2.1";
    let mut b = cc::Build::new();
    b.include("shim/include-gen").include(format!("{vendor}/include")).include(format!("{vendor}/libspeex")).include("shim");
    for s in SOURCES {
        b.file(format!("{vendor}/libspeex/{s}"));
    }
    b.file("shim/nhlms_speex_shim.c");
    b.define("FLOATING_POINT", None).define("USE_ALLOCA", None).define("EXPORT", Some("")).define("OS_SUPPORT_CUSTOM", None);
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        b.define("inline", Some("__inline"));
    }
    b.warnings(false);
    b.compile("nhlms_speex");
    println!("cargo:rerun-if-changed=shim");
    println!("cargo:rerun-if-changed={vendor}");
    println!("cargo:rerun-if-changed=build.rs");
}
