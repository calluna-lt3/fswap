extern crate pathdiff;

use std::collections::VecDeque;
use std::env;
use std::ffi::OsString;
use std::fs::{self, DirEntry, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str;

const FSWAP_EXT: &'static str = ".fswap";

// contains: if None, returns every file
fn find_files_with(path: &PathBuf, contains: Option<&str>) -> Option<Vec<String>> {
    let contains = match contains {
        Some(x) => x,
        None => &"",
    };

    let start_files = fs::read_dir(&path).unwrap_or_else(|err| {
        eprintln!(
            "ERROR: Couldn't read dir '{dir}': {err}",
            dir = path.display()
        );
        exit(1);
    });

    let mut files: VecDeque<DirEntry> = VecDeque::from(
        start_files
            .map(|x| {
                x.unwrap_or_else(|err| {
                    eprintln!("ERROR: Unhandleable fs::DirEntry error: {err}");
                    exit(1);
                })
            })
            .collect::<Vec<DirEntry>>(),
    );
    let mut fswap_files: Vec<String> = vec![];

    loop {
        let file = match files.pop_front() {
            Some(x) => x,
            None => break,
        };

        let file_md = file.metadata().unwrap_or_else(|err| {
            eprintln!(
                "ERROR: Couldn't get metadata from '{file}': {err}",
                file = file.path().display()
            );
            exit(1);
        });

        if file_md.is_dir() {
            let sub_files = fs::read_dir(file.path()).unwrap_or_else(|err| {
                eprintln!(
                    "ERROR: Couldn't read dir '{dir}': {err}",
                    dir = file.path().display()
                );
                exit(1);
            });

            sub_files.into_iter().for_each(|file| {
                files.push_back(file.unwrap_or_else(|err| {
                    eprintln!("ERROR: Unhandleable fs::DirEntry error: {err}");
                    exit(1);
                }))
            });
        } else {
            let file_path = file.path(); // idk why this is necessary at all, but borrow checker doesnt like it if i dont do this
            let file_path_str = file_path.to_str().unwrap_or_else(|| {
                eprintln!(
                    "ERROR: '{file}' is invalid unicode.",
                    file = file_path.display()
                );
                exit(1);
            });
            if file_path_str.contains(contains) {
                fswap_files.push(String::from(file_path_str));
            }
        }
    }

    if fswap_files.len() > 0 {
        Some(fswap_files)
    } else {
        None
    }
}

fn confirm_cmd(description: &String) -> bool {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("CONFIRM: {description}? [y/N] ");
        stdout.flush().unwrap_or_else(|err| {
            eprintln!("ERROR: Unhandleable io::stdout() error: {err}");
            exit(1);
        });
        let mut buf = String::new();
        match stdin.read_line(&mut buf) {
            Ok(n) if n == 2 => {
                let buf = buf.trim().to_ascii_lowercase();
                if buf.eq("y") {
                    return true;
                } else if buf.eq("n") {
                    return false;
                } else {
                    continue;
                }
            }
            Ok(_) => {
                return false;
            }
            Err(err) => {
                eprintln!("ERROR: Couldn't read line: {err}");
                exit(1);
            }
        };
    }
}

fn cmd_begin(u_input: &mut UserInput) -> bool {
    let arg = u_input.next_arg();
    let source_dir = Path::new(&arg);

    let arg: String;
    if u_input.argc > 0 {
        arg = u_input.next_arg();
    } else {
        arg = String::from(".");
    }

    let working_dir = Path::new(&arg);

    // Check both files exist
    if !source_dir.exists() {
        eprintln!("ERROR: '{dir}' doesn't exist.", dir = source_dir.display());
        exit(1);
    };

    if !working_dir.exists() {
        eprintln!("ERROR: '{dir}' doesn't exist.", dir = working_dir.display());
        exit(1);
    }

    // Check files are directories
    let source_md = source_dir.metadata().unwrap_or_else(|err| {
        eprintln!(
            "ERROR: Couldn't get metadata from '{dir}': {err}",
            dir = source_dir.display()
        );
        exit(1);
    });

    if !source_md.file_type().is_dir() {
        eprintln!(
            "ERROR: '{dir}' isn't a directory.",
            dir = source_dir.display()
        );
        exit(1);
    }

    let working_md = working_dir.metadata().unwrap_or_else(|err| {
        eprintln!(
            "ERROR: Couldn't get metadata from '{dir}': {err}",
            dir = working_dir.display()
        );
        exit(1);
    });
    if !working_md.file_type().is_dir() {
        eprintln!(
            "ERROR: '{dir}' isn't a directory.",
            dir = working_dir.display()
        );
        exit(1);
    }

    // Create and populate .fswap file
    let mut fswap_path = working_dir.to_path_buf();
    fswap_path.push(FSWAP_EXT);

    let mut fswap_file = File::create_new(&fswap_path).unwrap_or_else(|err| {
        eprintln!(
            "ERROR: Couldn't create '{file}': {err}",
            file = fswap_path.display()
        );
        exit(1);
    });

    let path_diff = pathdiff::diff_paths(source_dir, working_dir).unwrap_or_else(|| {
        panic!(
            "ERROR: pathdiff::diff_paths returned None.\nI could never reach this through testing."
        );
    });

    if path_diff == PathBuf::from("") {
        eprintln!("ERROR: Source directory and fswap directory cannot be the same.");
        exit(1);
    }

    if let Err(err) = write!(fswap_file, "{src}", src = path_diff.as_path().display()) {
        eprintln!(
            "ERROR: couldn't write to '{file}': {err}",
            file = fswap_path.display()
        );
        exit(1);
    }

    if u_input.opts.verbose {
        println!(
            "INFO: Created file '{file}', with path to source '{path}'.",
            file = fswap_path.display(),
            path = path_diff.display()
        );
    }

    return true;
}

fn cmd_info(u_input: &mut UserInput) -> bool {
    let arg: String;
    if u_input.argc > 0 {
        arg = u_input.next_arg();
    } else {
        arg = String::from(".");
    }

    let mut working_dir = PathBuf::from(arg);

    working_dir.push(FSWAP_EXT);
    if let Err(err) = File::open(&working_dir) {
        eprintln!(
            "ERROR: Couldn't open '{file}': {err}",
            file = working_dir.display()
        );
        exit(1);
    }

    working_dir.pop();

    match find_files_with(&working_dir, Some(FSWAP_EXT)) {
        Some(paths) => {
            println!("fswap files in '{dir}':", dir = working_dir.display());
            paths.iter().for_each(|x| println!("  {x}"));
        }
        None => {
            println!("No fswap files in '{dir}'.", dir = working_dir.display());
        }
    };

    return true;
}

fn cmd_help(u_input: &mut UserInput) -> bool {
    let mut arg = String::from("none");
    if u_input.argc > 0 {
        arg = u_input.next_arg();
    }

    let help = match arg.as_str() {
        "begin"  => String::from("Usage: fswap begin [SOURCE DIR] [FSWAP DIR]\nCreates .fswap file linking SOURCE DIR and FSWAP DIR."),
        "end"    => String::from("Usage: fswap end [FSWAP DIR]\nDeletes .fswap file, and ALL swapped files. Does not revert changes before doing so."),
        "help"   => String::from("Usage: fswap help [COMMAND]\nPrints a brief description of what COMMAND does."),
        "info"   => String::from("Usage: fswap info [FSWAP DIR]\nPrints all swapped files."),
        "revert" => String::from("Usage: fswap revert [FILES...]\nReverts a swapped file to it's original state (e.g. file.fswap -> file)."),
        "swap"   => String::from("Usage: fswap swap [FILES...]\nIf they both exist, swaps a file from SOURCE DIR to FSWAP DIR, and saves the swapped file."),
        "none"   => format!(r#"Usage: fswap [COMMAND] [OPTIONS...] [ARGUMENTS...]

NOTE: FSWAP DIR is an optional argument, and defaults to the current working directory.

COMMANDS
  b[egin]   [SOURCE DIR] [FSWAP DIR]    creates .fswap file linking SOURCE_DIR and FSWAP DIR
  e[nd]     [FSWAP DIR]                 deletes .fswap file, and ALL swapped files
  i[nfo]    [FSWAP DIR]                 prints all swapped files
  r[evert]  [FILES...]                  reverts a swapped file to it's original state (e.g. file.fswap -> file)
  s[wap]    [FILES...]                  if they both exist, swaps a file from SOURCE DIR to FSWAP DIR, and saves the swapped file

OPTIONS
  -a, --all          do command to all files in fswap directory
  -h, --help         print this help, or help of another command
  -n, --noconfirm    will not ask for confirmation to overwrite files
  -r, --recursive    do command to all files in directory
  -v, --verbose      prints everything given command does"#),
        _ => {
            eprintln!("ERROR: Cannot provide help for unknown command: {arg}");
            exit(1);
        }
    };

    println!("{}", help);
    return true;
}

fn cmd_end(u_input: &mut UserInput) -> bool {
    let arg: String;
    if u_input.argc > 0 {
        arg = u_input.next_arg();
    } else {
        arg = String::from(".");
    }

    let path = PathBuf::from(&arg);

    if !u_input.opts.noconfirm {
        let confirmed = confirm_cmd(&format!("Delete all files with the suffix '{FSWAP_EXT}'"));
        if !confirmed {
            return true;
        }
    }

    let files = match find_files_with(&path, Some(FSWAP_EXT)) {
        Some(x) => x,
        None => return true,
    };

    for file in files {
        if let Err(err) = fs::remove_file(&file) {
            eprintln!("ERROR: Couldn't delete '{file}': {err}");
            exit(1);
        }

        if u_input.opts.verbose {
            println!("INFO: Deleted '{file}'.");
        }
    }

    return true;
}

fn cmd_swap(u_input: &mut UserInput) -> bool {
    let mut fswap_file = File::open(FSWAP_EXT).unwrap_or_else(|err| {
        eprintln!("ERROR: Couldn't open .fswap file: {err}");
        exit(1);
    });

    // Read .fswap file to find src directory
    let mut buf: [u8; 128] = [0; 128];
    let sd_len = fswap_file.read(&mut buf).unwrap_or_else(|err| {
        eprintln!("ERROR: Couldn't read .fswap file: {err}");
        exit(1);
    });

    let sd = str::from_utf8(&buf).unwrap_or_else(|err| {
        eprintln!("ERROR: Unhandleable str::from_utf8 error: {err}");
        exit(1);
    });
    let source_dir = Path::new(&sd[0..sd_len]);

    if !source_dir.exists() {
        eprintln!("ERROR: '{dir}' does not exist.", dir = source_dir.display());
        exit(1);
    }

    let files: Vec<String>;
    if u_input.opts.all {
        let swapped_files = match find_files_with(&PathBuf::from("."), None) {
            Some(x) => x,
            None => {
                eprintln!("No files found.");
                return true;
            }
        };

        // Filter out any fswap file
        files = swapped_files
            .into_iter()
            .filter(|x| !x.contains(FSWAP_EXT))
            .collect();
    } else if u_input.opts.recursive {
        let dirs = &u_input.args;
        let mut out_files: Vec<String> = vec![];
        for dir in dirs {
            let swapped_files = match find_files_with(&PathBuf::from(&dir), None) {
                Some(x) => x,
                None => {
                    eprintln!("No fswap files found.");
                    return true;
                }
            };

            out_files = combine_string_vecs(&out_files, &swapped_files);
        }

        // Filter out any fswap file
        files = out_files
            .into_iter()
            .filter(|x| !x.contains(FSWAP_EXT))
            .collect();
    } else {
        files = u_input.args.clone();
    }

    let mut files = files.iter();
    loop {
        let arg = match files.next() {
            Some(x) => x,
            None => break,
        };

        let working_file = PathBuf::from(&arg);
        let mut source_file = PathBuf::from(source_dir);
        source_file.push(&working_file);

        if !working_file.exists() {
            eprintln!(
                "ERROR: '{file}' doesn't exist.",
                file = working_file.display()
            );
            exit(1);
        }

        let working_md = working_file.metadata().unwrap_or_else(|err| {
            eprintln!(
                "ERROR: Couldn't get metadata from '{file}': {err}",
                file = working_file.display()
            );
            exit(1);
        });

        if !working_md.file_type().is_file() {
            eprintln!(
                "ERROR: '{file}' isn't a normal file.",
                file = working_file.display()
            );
            exit(1);
        }

        if !source_file.exists() {
            eprintln!(
                "ERROR: '{file}' does not exist.",
                file = source_file.display()
            );
            exit(1);
        }

        let swapped_file = append_to_pathbuf(&working_file, FSWAP_EXT);

        if !u_input.opts.noconfirm && swapped_file.exists() {
            let confirmed = confirm_cmd(&format!(
                "'{file}' already exists, overwrite this file",
                file = swapped_file.display()
            ));
            if !confirmed {
                continue;
            }
        }

        if let Err(err) = fs::rename(&working_file, &swapped_file) {
            eprintln!(
                "ERROR: Couldn't rename '{src}' to '{dest}': {err}",
                src = working_file.display(),
                dest = swapped_file.display()
            );
            exit(1);
        }

        if let Err(err) = fs::copy(&source_file, &working_file) {
            eprintln!(
                "ERROR: Couldn't copy '{src}' to '{dest}': {err}",
                src = source_file.display(),
                dest = working_file.display()
            );
            exit(1);
        }

        if u_input.opts.verbose {
            println!(
                "INFO: Renamed '{src}' -> '{dest}'.",
                src = working_file.display(),
                dest = swapped_file.display()
            );
            println!(
                "INFO: Copied '{src}' -> '{dest}'.",
                src = source_file.display(),
                dest = working_file.display()
            );
        }
    }

    return true;
}

fn cmd_revert(u_input: &mut UserInput) -> bool {
    if let Err(err) = File::open(FSWAP_EXT) {
        eprintln!("ERROR: Couldn't open '{FSWAP_EXT}': {err}");
        exit(1);
    }

    let files: Vec<String>;
    if u_input.opts.all {
        let swapped_files = match find_files_with(&PathBuf::from("."), Some(FSWAP_EXT)) {
            Some(x) => x,
            None => {
                eprintln!("No fswap files found.");
                return true;
            }
        };

        files = swapped_files
            .iter()
            .map(|x| {
                x.strip_suffix(FSWAP_EXT)
                    .expect("Files that contain .fswap should always have it as a suffix")
                    .to_string()
            })
            .filter(|x| !x.eq("./"))
            .collect();
    } else if u_input.opts.recursive {
        let dirs = &u_input.args;
        let mut out_files: Vec<String> = vec![];
        for dir in dirs {
            let swapped_files = match find_files_with(&PathBuf::from(&dir), Some(FSWAP_EXT)) {
                Some(x) => x,
                None => {
                    eprintln!("No fswap files found.");
                    return true;
                }
            };

            let cur_files = swapped_files
                .iter()
                .map(|x| {
                    x.strip_suffix(FSWAP_EXT)
                        .expect("Files that contain .fswap should always have it as a suffix")
                        .to_string()
                })
                .filter(|x| !x.eq("./"))
                .collect();
            out_files = combine_string_vecs(&out_files, &cur_files);
        }

        files = out_files;
    } else {
        files = u_input.args.clone();
    }

    let mut files = files.iter();
    loop {
        let file = match files.next() {
            Some(x) => x,
            None => break,
        };

        // Misleading name, in this case source_file actually refers to the file that was swapped
        // in FROM the source directory, not a file in the source directory
        let source_file = PathBuf::from(&file);

        if source_file.exists() {
            if let Err(err) = fs::remove_file(&source_file) {
                eprintln!(
                    "ERROR: couldn't remove '{file}': {err}",
                    file = source_file.display()
                );
                exit(1);
            }
        }

        let working_file = append_to_pathbuf(&source_file, FSWAP_EXT);
        if !working_file.exists() {
            eprintln!(
                "ERROR: '{file}' doesn't exist.",
                file = working_file.display()
            );
            exit(1);
        }

        if let Err(err) = fs::rename(&working_file, &source_file) {
            eprintln!(
                "ERROR: Couldn't rename '{work_file}' to '{src_file}': {err}",
                work_file = working_file.display(),
                src_file = source_file.display()
            );
            exit(1);
        }

        if u_input.opts.verbose {
            println!("INFO: Removed '{file}'.", file = source_file.display());
            println!(
                "INFO: Renamed '{src}' -> '{dest}'.",
                src = working_file.display(),
                dest = source_file.display()
            );
        }
    }

    return true;
}

fn combine_string_vecs(a: &Vec<String>, b: &Vec<String>) -> Vec<String> {
    let mut ret = a.clone();
    b.iter().for_each(|x| ret.push(x.to_string()));
    return ret;
}

// https://internals.rust-lang.org/t/pathbuf-has-set-extension-but-no-add-extension-cannot-cleanly-turn-tar-to-tar-gz/14187/11
fn append_to_pathbuf(pb: &PathBuf, ext: &str) -> PathBuf {
    let mut path: OsString = pb.clone().into();
    path.push(ext);
    path.into()
}

// wrapper for string.strip_suffix()
fn _strip_suffix_from_pathbuf(pb: &PathBuf, ext: &str) -> Option<PathBuf> {
    let path = String::from(pb.clone().into_os_string().into_string().unwrap());
    match path.strip_suffix(ext) {
        Some(x) => {
            return Some(PathBuf::from(x));
        }
        None => {
            return None;
        }
    };
}

struct Opts {
    all: bool,
    help: bool,
    noconfirm: bool,
    recursive: bool,
    verbose: bool,
}

impl Opts {
    fn new() -> Self {
        Self {
            all: false,
            help: false,
            noconfirm: false,
            recursive: false,
            verbose: false,
        }
    }
}

// idk if this level of abstraction is even good, but did it 4 learning purposes
struct UserInput {
    args: Vec<String>,
    argc: usize,
    opts: Opts,
}

impl UserInput {
    fn new() -> Self {
        let mut args_in: Vec<String> = env::args().collect();
        args_in.remove(0);
        let opts_out = Self::args_to_flags(&args_in);
        let args_out = Self::strip_opts_from_args(&args_in);
        let argc_out = args_out.len();

        return Self {
            args: args_out,
            argc: argc_out,
            opts: opts_out,
        };
    }

    fn next_arg(&mut self) -> String {
        if self.argc < 1 {
            Self::usage();
            exit(1);
        }

        self.argc -= 1;
        return self.args.remove(0);
    }

    fn usage() {
        println!("Usage: fswap [COMMAND] [OPTIONS...] [ARGUMENTS...]\nSee 'fswap help' for more information.");
    }

    fn strip_opts_from_args(args: &Vec<String>) -> Vec<String> {
        // IMPORTANT NOTE: '⟡' looks like a face
        return args
            .clone()
            .into_iter()
            .filter(|x| x.chars().next().unwrap_or('⟡') != '-')
            .collect();
    }

    fn args_to_flags(args: &Vec<String>) -> Opts {
        let mut opts = Opts::new();
        for arg in args {
            let mut chars = arg.chars().peekable();
            match chars.next() {
                Some(c) if c == '-' => c,
                _ => continue,
            };

            // peek here so it wont consume an important character valid in case of short opt
            let long_opt = match chars.peek() {
                Some(c) if *c == '-' => true,
                _ => false,
            };

            if !long_opt {
                loop {
                    match chars.next() {
                        Some(c) if c == 'a' => opts.all = true,
                        Some(c) if c == 'h' => opts.help = true,
                        Some(c) if c == 'n' => opts.noconfirm = true,
                        Some(c) if c == 'r' => opts.recursive = true,
                        Some(c) if c == 'v' => opts.verbose = true,
                        Some(_) => {
                            eprintln!("ERROR: Invalid options '{arg}'");
                            exit(1);
                        }
                        None => break,
                    };
                }
            } else {
                match arg.as_str() {
                    "--all" => opts.all = true,
                    "--help" => opts.help = true,
                    "--noconfirm" => opts.noconfirm = true,
                    "--recursive" => opts.recursive = true,
                    "--verbose" => opts.verbose = true,
                    _ => {
                        eprintln!("ERROR: Invalid options '{arg}'");
                        exit(1);
                    }
                };
            }
        }

        return opts;
    }
}

struct Command {
    name: &'static str,
    short: &'static str,
    func: fn(u_input: &mut UserInput) -> bool,
}

// code style inspired by https://github.com/rexim/tore
// tool usage inspired by nmcli
const COMMANDS: [Command; 6] = [
    Command {
        name: "begin",
        short: "b",
        func: cmd_begin,
    },
    Command {
        name: "end",
        short: "e",
        func: cmd_end,
    },
    Command {
        name: "help",
        short: "h",
        func: cmd_help,
    },
    Command {
        name: "info",
        short: "i",
        func: cmd_info,
    },
    Command {
        name: "revert",
        short: "r",
        func: cmd_revert,
    },
    Command {
        name: "swap",
        short: "s",
        func: cmd_swap,
    },
];

// Commands either succeed and return, or exit the program with non-zero exit code
fn main() {
    let mut u_input = UserInput::new();

    if u_input.opts.help {
        cmd_help(&mut u_input);
        exit(0);
    }

    let mut command = String::from("info");
    if u_input.argc > 0 {
        command = u_input.next_arg();
    }

    for cmd in COMMANDS.iter() {
        if cmd.name.eq(&command) || cmd.short.eq(&command) {
            if (cmd.func)(&mut u_input) {
                exit(0)
            } else {
                exit(1)
            }
        }
    }

    eprintln!("ERROR: Unknown command '{command}'");
    UserInput::usage();
    exit(1);
}
