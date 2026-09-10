//! silicon — compile a nox formula to native code
//!
//! Standalone binary: one program speaks nox, this walks its formula tree
//! and writes bytes/text for one machine. No execution, no proving — see
//! the crate-level docs.

use clap::Parser;
use nebu::Goldilocks;
use nox::{Order as NounId, Reduction as Order};

const ORDER_SIZE: usize = 1 << 16; // 64K nouns

#[derive(Parser)]
#[command(name = "silicon", about = "Compile a nox formula to native code — 25 backends")]
struct Cli {
    /// Formula file (.nox) or inline with -e
    #[arg()]
    file: Option<String>,

    /// Inline formula
    #[arg(short = 'e')]
    expr: Option<String>,

    /// Target: wasm, arm64, x64, x64-sysv, rv32, rv64, rvv, thumb2, hexagon, ebpf, ptx, ptx-parallel, tensor-cores, wgsl, spirv, ane, ane-batch, amx, intel-amx, xla, onnx, cerebras, upmem, qasm, qir, verilog, systemverilog, vhdl
    #[arg(short = 't', long = "target", default_value = "wasm")]
    target: String,

    /// Number of parameters
    #[arg(short = 'p', long = "params", default_value = "2")]
    params: u32,

    /// Output file (default: stdout for text, stderr summary for binary)
    #[arg(short = 'o')]
    output: Option<String>,
}

fn main() {
    let args = Cli::parse();

    let formula_text = match (&args.file, &args.expr) {
        (_, Some(e)) => e.clone(),
        (Some(f), None) => std::fs::read_to_string(f).unwrap_or_else(|e| {
            eprintln!("error reading '{}': {}", f, e);
            std::process::exit(1);
        }),
        (None, None) => {
            use std::io::IsTerminal;
            use std::io::Read;
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
            let wasm = trident_silicon::wasm::compile_to_wasm(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&wasm, &output_path, "wasm");
        }
        "arm64" => {
            let code = trident_silicon::arm64::compile_to_arm64(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "arm64");
        }
        "x64" | "x86-64" | "x86_64" => {
            let code = trident_silicon::x86_64::compile_to_x86_64(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "x86-64 (win64)");
        }
        "x64-sysv" => {
            let code = trident_silicon::x86_64::compile_to_x86_64_sysv(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "x86-64 (sysv)");
        }
        "rv64" => {
            let code = trident_silicon::rv64::compile_to_rv64(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "rv64");
        }
        "ebpf" => {
            let code = trident_silicon::ebpf::compile_to_ebpf(&order, formula, num_params)
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
            let ptx = trident_silicon::ptx::compile_to_ptx(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&ptx, &output_path);
        }
        "ptx-parallel" | "cuda-parallel" => {
            let ptx = trident_silicon::ptx::compile_to_ptx_parallel(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&ptx, &output_path);
        }
        "wgsl" | "wgpu" | "webgpu" => {
            let wgsl = trident_silicon::wgsl::compile_to_wgsl(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&wgsl, &output_path);
        }
        "spirv" | "spir-v" | "vulkan" => {
            let spv = trident_silicon::spirv::compile_to_spirv(&order, formula, num_params)
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
            let mil = trident_silicon::ane::compile_to_mil(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&mil, &output_path);
        }
        "ane-batch" | "mil-batch" => {
            let mil = trident_silicon::ane::compile_to_mil_batch(&order, formula, num_params, 256)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&mil, &output_path);
        }
        "rv32" | "riscv32" | "esp32" => {
            let code = trident_silicon::rv32::compile_to_rv32(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "rv32");
        }
        "thumb2" | "cortex-m" | "stm32" | "rp2040" => {
            let code = trident_silicon::thumb2::compile_to_thumb2(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_binary(&code, &output_path, "thumb2");
        }
        "verilog" | "fpga" | "hdl" => {
            let v = trident_silicon::verilog::compile_to_verilog(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "qasm" | "openqasm" | "quantum" => {
            let q = trident_silicon::qasm::compile_to_qasm(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&q, &output_path);
        }
        "qir" | "azure-quantum" => {
            let q = trident_silicon::qir::compile_to_qir(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&q, &output_path);
        }
        "onnx" => {
            let o = trident_silicon::onnx::compile_to_onnx(&order, formula, num_params)
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
            let x = trident_silicon::xla::compile_to_xla(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&x, &output_path);
        }
        "systemverilog" | "sv" | "asic" => {
            let v = trident_silicon::systemverilog::compile_to_systemverilog(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "vhdl" => {
            let v = trident_silicon::vhdl::compile_to_vhdl(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "amx" | "apple-amx" => {
            let v = trident_silicon::amx::compile_to_amx(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "intel-amx" | "amx-intel" => {
            let v = trident_silicon::intel_amx::compile_to_amx(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "tensor-cores" | "wmma" => {
            let v = trident_silicon::tensor_cores::compile_to_tensor_ptx(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "hexagon" | "qdsp" | "dsp" => {
            let v = trident_silicon::hexagon::compile_to_hexagon(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "rvv" | "riscv-vector" => {
            let v = trident_silicon::rvv::compile_to_rvv(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "cerebras" | "csl" | "wafer" => {
            let v = trident_silicon::cerebras::compile_to_csl(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        "upmem" | "pim" => {
            let v = trident_silicon::upmem::compile_to_upmem(&order, formula, num_params)
                .unwrap_or_else(|e| { eprintln!("compile error: {:?}", e); std::process::exit(1); });
            write_text(&v, &output_path);
        }
        other => {
            eprintln!("unknown target: {}", other);
            eprintln!("supported: wasm arm64 x64 x64-sysv rv64 rv32 rvv ebpf thumb2 hexagon ptx ptx-parallel tensor-cores wgsl spirv ane ane-batch amx intel-amx verilog systemverilog vhdl qasm qir onnx xla cerebras upmem");
            std::process::exit(1);
        }
    }
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
            order.atom(Goldilocks::new(*v))
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
                result = order.pair(head, result)
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
