//! Standalone native command-line interface.
use diagram_core::Theme;
use diagram_render::Font;
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};
const HELP: &str = "diagram-rs — native Mermaid diagrams to SVG\n\nUsage:\n  diagram-rs [render] INPUT [-o OUTPUT] [--theme light|dark] [--font FONT.ttf]\n  diagram-rs check INPUT\n  diagram-rs --help\n  diagram-rs --version\n\nINPUT may be - for stdin. SVG is written to stdout unless -o is given.\ncheck parses and validates supported syntax; render also validates font coverage.\nNo Node.js, browser, or network is required.\n";
fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("diagram-rs: {e}");
            ExitCode::FAILURE
        }
    }
}
fn run(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        print!("{HELP}");
        return Ok(());
    }
    if args == ["--version"] {
        println!("diagram-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut check = false;
    let mut input = None;
    let mut output = None;
    let mut theme = None;
    let mut font_path = None;
    let mut i = 0;
    if args[0] == "check" {
        check = true;
        i += 1;
    } else if args[0] == "render" {
        i += 1;
    }
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" | "--theme" | "--font" => {
                let flag = &args[i];
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("{flag} requires a value"))?;
                match flag.as_str() {
                    "--theme" => {
                        theme = Some(match value.as_str() {
                            "light" => Theme::Light,
                            "dark" => Theme::Dark,
                            _ => return Err("theme must be light or dark".into()),
                        })
                    }
                    "--font" => font_path = Some(PathBuf::from(value)),
                    _ => output = Some(PathBuf::from(value)),
                }
            }
            "-" => {
                if input.replace("-".to_string()).is_some() {
                    return Err("only one input is allowed".into());
                }
            }
            s if s.starts_with('-') => return Err(format!("unknown option: {s}; see --help")),
            s => {
                if input.replace(s.to_string()).is_some() {
                    return Err("only one input is allowed".into());
                }
            }
        }
        i += 1;
    }
    let input = input.ok_or("missing input; see --help")?;
    if check && (output.is_some() || theme.is_some() || font_path.is_some()) {
        return Err("check accepts only INPUT".into());
    }
    if output.as_ref().is_some_and(|p| p == &PathBuf::from(&input)) {
        return Err("input and output paths must differ".into());
    }
    if input != "-"
        && let Some(path) = &output
        && let (Ok(source_path), Ok(output_path)) =
            (fs::canonicalize(&input), fs::canonicalize(path))
        && source_path == output_path
    {
        return Err("input and output refer to the same file".into());
    }
    let mut bytes = Vec::new();
    if input == "-" {
        io::stdin()
            .take((diagram_core::MAX_SOURCE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
    } else {
        fs::File::open(&input)
            .map_err(|e| format!("{input}: {e}"))?
            .take((diagram_core::MAX_SOURCE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
    }
    if bytes.len() > diagram_core::MAX_SOURCE_BYTES {
        return Err("input exceeds 128 KiB".into());
    }
    let source = String::from_utf8(bytes).map_err(|_| "input is not UTF-8")?;
    let mut graph = diagram_parser::parse_diagram(&source).map_err(|e| format!("{input}:{e}"))?;
    if check {
        println!("{input}: valid {} ({})", graph.kind(), graph.summary());
        return Ok(());
    }
    if let Some(theme) = theme {
        graph.config_mut().theme = theme;
    }
    let svg = if let Some(path) = font_path {
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let font = Font::from_bytes(&bytes)?;
        diagram_render::render_diagram_with_font(&graph, &font)
    } else {
        diagram_render::render_diagram(&graph)
    }
    .map_err(|e| format!("{input}:{e}"))?;
    if let Some(path) = output {
        fs::write(&path, svg).map_err(|e| format!("{}: {e}", path.display()))?;
    } else {
        io::stdout()
            .write_all(svg.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
