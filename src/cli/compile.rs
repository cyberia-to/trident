//! trident compile — compile nox formula to native code
//!
//! Moved from nox CLI. Takes a nox formula (text) and compiles it
//! to one of: wasm, arm64, x64, rv64, ebpf, ptx, wgsl, spirv, ane.

use clap::Args;
use nebu::Goldilocks;
use nox::noun::{Order, NounId, Noun, Tag};

const ORDER_SIZE: usize = 1 << 16; // 64K nouns

#[derive(Args)]
pub struct CompileNoxArgs {
    /// Formula file (.nox) or inline with -e
    #[arg()]
    file: Option<String>,

    /// Inline formula
    #[arg(short = 'e')]
    expr: Option<String>,

    /// Target: wasm, arm64, x64, x64-sysv, rv64, ebpf, ptx, ptx-parallel, wgsl, spirv, ane, ane-batch
    #[arg(short = 't', long = "target", default_value = "wasm")]
    target: String,

    /// Number of parameters
    #[arg(short = 'p', long = "params", default_value = "2")]
    params: u32,

    /// Output file (default: stdout for text, stderr summary for binary)
    #[arg(short = 'o')]
    output: Option<String>,
}

pub fn cmd_compile_nox(args: CompileNoxArgs) {
    let formula_text = match (&args.file, &args.expr) {
        (_, Some(e)) => e.clone(),
        (Some(f), None) => std::fs::read_to_string(f).unwrap_or_else(|e| {
            eprintln!("error reading '{}': {}", f, e);
            std::process::exit(1);
        }),
        (None, None) => {
            use std::io::Read;
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                eprintln!("error: no formula provided. Use -e or pass a .nox file.");
                std::process::exit(1);
            }
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                eprintln!("error reading stdin: {}", e);
                std::process::exit(1);
            });
            buf
        }
    };

    let formula_text = formula_text.trim();
    if formula_text.is_empty() {
        eprintln!("error: no formula provided");
        std::process::exit(1);
    }

    let mut order = Order::<ORDER_SIZE>::new();
    let formula = parse_noun(&mut order, formula_text).unwrap_or_else(|e| {
        eprintln!("error parsing formula: {}", e);
        std::process::exit(1);
    });

    let num_params = args.params;
    let output_path = args.output.unwrap_or_default();

    match args.target.as_str() {
        "wasm" => {
            let wasm = trident::compile::wasm::compile_to_wasm(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&wasm, &output_path, "wasm");
        }
        "arm64" => {
            let code = trident::compile::arm64::compile_to_arm64(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "arm64");
        }
        "x64" | "x86-64" | "x86_64" => {
            let code = trident::compile::x86_64::compile_to_x86_64(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "x86-64 (win64)");
        }
        "x64-sysv" => {
            let code = trident::compile::x86_64::compile_to_x86_64_sysv(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "x86-64 (sysv)");
        }
        "rv64" => {
            let code = trident::compile::rv64::compile_to_rv64(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "rv64");
        }
        "ebpf" => {
            let code = trident::compile::ebpf::compile_to_ebpf(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            if output_path.is_empty() {
                eprintln!("ebpf: {} bytes ({} insns)", code.len(), code.len() / 8);
                for (i, chunk) in code.chunks(8).enumerate() {
                    let insn = u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8]));
                    eprintln!("  {:3}: {:016x}", i, insn);
                }
            } else {
                write_file(&code, &output_path);
            }
        }
        "ptx" | "cuda" => {
            let ptx = trident::compile::ptx::compile_to_ptx(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&ptx, &output_path);
        }
        "ptx-parallel" | "cuda-parallel" => {
            let ptx = trident::compile::ptx::compile_to_ptx_parallel(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&ptx, &output_path);
        }
        "wgsl" | "wgpu" | "webgpu" => {
            let wgsl = trident::compile::wgsl::compile_to_wgsl(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&wgsl, &output_path);
        }
        "spirv" | "spir-v" | "vulkan" => {
            let spv = trident::compile::spirv::compile_to_spirv(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            if output_path.is_empty() {
                eprintln!("spir-v: {} bytes", spv.len());
                use std::io::Write;
                std::io::stdout().write_all(&spv).unwrap();
            } else {
                write_file(&spv, &output_path);
            }
        }
        "ane" | "mil" | "neural-engine" => {
            let mil = trident::compile::ane::compile_to_mil(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&mil, &output_path);
        }
        "ane-batch" | "mil-batch" => {
            let mil = trident::compile::ane::compile_to_mil_batch(&order, formula, num_params, 256)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&mil, &output_path);
        }
        "rv32" | "riscv32" | "esp32" => {
            let code = trident::compile::rv32::compile_to_rv32(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "rv32");
        }
        "thumb2" | "cortex-m" | "stm32" | "rp2040" => {
            let code = trident::compile::thumb2::compile_to_thumb2(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "thumb2");
        }
        "verilog" | "fpga" | "hdl" => {
            let v = trident::compile::verilog::compile_to_verilog(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "qasm" | "openqasm" | "quantum" => {
            let q = trident::compile::qasm::compile_to_qasm(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&q, &output_path);
        }
        "qir" | "azure-quantum" => {
            let q = trident::compile::qir::compile_to_qir(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&q, &output_path);
        }
        "onnx" => {
            let o = trident::compile::onnx::compile_to_onnx(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            if output_path.is_empty() {
                eprintln!("onnx: {} bytes", o.len());
                use std::io::Write;
                std::io::stdout().write_all(&o).unwrap();
            } else {
                write_file(&o, &output_path);
            }
        }
        "xla" | "hlo" | "tpu" => {
            let x = trident::compile::xla::compile_to_xla(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&x, &output_path);
        }
        other => {
            eprintln!("unknown target: {}", other);
            eprintln!("supported: wasm, arm64, x64, x64-sysv, rv64, rv32, ebpf, thumb2, ptx, ptx-parallel, wgsl, spirv, ane, ane-batch, verilog, qasm, qir, onnx, xla");
            std::process::exit(1);
        }
    }
}

// ─── MIR subcommand ─────────────────────────────────────────────

#[derive(Args)]
pub struct MirArgs {
    /// MIR JSON file (or - for stdin)
    #[arg()]
    file: Option<String>,

    /// Compile only this function
    #[arg(short = 'f', long = "function")]
    function: Option<String>,

    /// Output directory for .nox files
    #[arg(short = 'o')]
    output_dir: Option<String>,
}

pub fn cmd_mir(args: MirArgs) {
    let mut raw_args: Vec<String> = Vec::new();
    if let Some(f) = args.file {
        raw_args.push(f);
    }
    if let Some(ref func) = args.function {
        raw_args.push("-f".to_string());
        raw_args.push(func.clone());
    }
    if let Some(ref dir) = args.output_dir {
        raw_args.push("-o".to_string());
        raw_args.push(dir.clone());
    }
    trident::compile::mir2nox::run_mir2nox(&raw_args);
}

// ─── noun parser (from nox CLI) ─────────────────────────────────

fn parse_noun(order: &mut Order<ORDER_SIZE>, input: &str) -> Result<NounId, String> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    parse_expr(order, &tokens, &mut pos)
}

#[derive(Debug)]
enum Token {
    Open,
    Close,
    Num(u64),
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            '[' => { tokens.push(Token::Open); chars.next(); }
            ']' => { tokens.push(Token::Close); chars.next(); }
            ' ' | '\t' | '\n' | '\r' => { chars.next(); }
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '_' {
                        if c != '_' { num.push(c); }
                        chars.next();
                    } else {
                        break;
                    }
                }
                let v: u64 = num.parse().map_err(|e| format!("bad number '{}': {}", num, e))?;
                tokens.push(Token::Num(v));
            }
            _ => { chars.next(); }
        }
    }
    Ok(tokens)
}

fn parse_expr(
    order: &mut Order<ORDER_SIZE>,
    tokens: &[Token],
    pos: &mut usize,
) -> Result<NounId, String> {
    if *pos >= tokens.len() {
        return Err("unexpected end of input".to_string());
    }
    match &tokens[*pos] {
        Token::Num(v) => {
            *pos += 1;
            order.atom(Goldilocks::new(*v), Tag::Field)
                .ok_or_else(|| "order full".to_string())
        }
        Token::Open => {
            *pos += 1;
            let mut elems = Vec::new();
            while *pos < tokens.len() && !matches!(tokens[*pos], Token::Close) {
                elems.push(parse_expr(order, tokens, pos)?);
            }
            if *pos < tokens.len() && matches!(tokens[*pos], Token::Close) {
                *pos += 1;
            } else {
                return Err("expected ']'".to_string());
            }
            if elems.is_empty() {
                return Err("empty brackets".to_string());
            }
            if elems.len() == 1 {
                return Ok(elems[0]);
            }
            let mut result = elems.pop().unwrap();
            while let Some(head) = elems.pop() {
                result = order.cell(head, result)
                    .ok_or_else(|| "order full".to_string())?;
            }
            Ok(result)
        }
        Token::Close => {
            Err("unexpected ']'".to_string())
        }
    }
}

// ─── output helpers ─────────────────────────────────────────────

fn write_binary(data: &[u8], path: &str, label: &str) {
    if path.is_empty() {
        eprintln!("{}: {} bytes of machine code", label, data.len());
        for (i, byte) in data.iter().enumerate() {
            if i % 16 == 0 && i > 0 { eprintln!(); }
            eprint!("{:02x} ", byte);
        }
        eprintln!();
    } else {
        write_file(data, path);
    }
}

fn write_text(text: &str, path: &str) {
    if path.is_empty() {
        print!("{}", text);
    } else {
        std::fs::write(path, text).unwrap_or_else(|e| {
            eprintln!("error writing '{}': {}", path, e);
            std::process::exit(1);
        });
        eprintln!("wrote {} bytes to {}", text.len(), path);
    }
}

fn write_file(data: &[u8], path: &str) {
    std::fs::write(path, data).unwrap_or_else(|e| {
        eprintln!("error writing '{}': {}", path, e);
        std::process::exit(1);
    });
    eprintln!("wrote {} bytes to {}", data.len(), path);
}
