extern crate pathdiff;

use std::env;
use std::str;
use std::process::exit;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use std::fs::{self, File, DirEntry};

const FSWAP_EXT: &'static str = ".fswap";

use bitflags::bitflags;
bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Opts: u32 {
        const NONE = 0;
        const ALL_FILES = 1 << 1;
        const HELP      = 1 << 2;
        const RECURSIVE = 1 << 3;
    }
}


fn cmd_begin(u_input: &mut UserInput) {

    let arg = u_input.next_arg();
    let source_dir = Path::new(&arg);
    let arg = u_input.next_arg();
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

    // Check files aren't directories
    let source_md = source_dir.metadata().unwrap_or_else(|err| {
        eprintln!("ERROR: Couldn't get metadata from '{dir}': {err}", dir = source_dir.display());
        exit(1);
    });

    if !source_md.file_type().is_dir() {
        eprintln!("ERROR: '{dir}' isn't a directory.", dir = source_dir.display());
        exit(1);
    }

    let working_md = working_dir.metadata().unwrap_or_else(|err| {
        eprintln!("ERROR: Couldn't get metadata from '{dir}': {err}", dir = working_dir.display());
        exit(1);
    });
    if !working_md.file_type().is_dir() {
        eprintln!("ERROR: '{dir}' isn't a directory.", dir = working_dir.display());
        exit(1);
    }

    // Create and populate .fswap file
    let mut fswap_path = working_dir.to_path_buf();
    fswap_path.push(FSWAP_EXT);

    let mut fswap_file = File::create_new(&fswap_path).unwrap_or_else(|err| {
        eprintln!("ERROR: Couldn't create '{file}': {err}", file = fswap_path.display());
        exit(1);
    });

    let path_diff = pathdiff::diff_paths(source_dir, working_dir).expect("pathdiff shouldnt fail");
    if let Err(err) = write!(fswap_file, "{}", path_diff.as_path().display()) {
        eprintln!("ERROR: couldn't write to '{file}': {err}", file = fswap_path.display());
        exit(1);
    }

}


fn find_fswap_files(path: &PathBuf) -> Option<Vec<String>> {
    let all_files = fs::read_dir(&path).unwrap_or_else(|err| {
        eprintln!("ERROR: Couldn't read dir '{dir}': {err}", dir = path.display());
        exit(1);
    });

    // NOTE: failing here shouldn't be my problem
    let mut files: VecDeque<DirEntry> = VecDeque::from(all_files.map(|x| x.unwrap()).collect::<Vec<DirEntry>>());
    let mut fswap_files: Vec<String> = vec![];

    loop {
        let file = match files.pop_front() {
            Some(x) => x,
            None    => break,
        };

        let file_md = file.metadata().unwrap_or_else(|err| {
            eprintln!("ERROR: Couldn't get metadata from '{file}': {err}", file = file.path().display());
            exit(1);
        });

        if file_md.is_dir() {
            let sub_files = fs::read_dir(file.path()).unwrap_or_else(|err| {
                eprintln!("ERROR: Couldn't read dir '{dir}': {err}", dir = file.path().display());
                exit(1);
            });

            sub_files.into_iter().for_each(|file| files.push_back(file.unwrap()));
        } else {
            let file_path = file.path();                     // idk why this is necessary at all, but borrow checker doesnt like it if i dont do this
            let file_path_str = file_path.to_str().unwrap(); // NOTE: failing here shouldn't be my problem
            if file_path_str.contains(FSWAP_EXT) && !file_path_str.eq("./.fswap") {
                fswap_files.push(String::from(file_path_str));
            }

        }
    }

    if fswap_files.len() == 0 {
        None
    } else {
        Some(fswap_files)
    }
}


fn cmd_info(u_input: &mut UserInput) {
    let arg: String;
    if u_input.argc > 0 {
        arg = u_input.next_arg();
    } else {
        arg = String::from(".");
    }

    let working_dir = PathBuf::from(arg);

    let mut fswap_file = working_dir.clone();
    fswap_file.push(FSWAP_EXT);
    if let Err(err) = File::open(&fswap_file) {
        eprintln!("ERROR: Couldn't open '{FSWAP_EXT}': {err}");
        exit(1);
    }

    match find_fswap_files(&working_dir) {
        Some(paths) => {
            println!("fswap files in '{dir}'", dir = working_dir.display());
            paths.iter().for_each(|x| println!("{x}"));
        },
        None => {
            eprintln!("No fswap files found.");
        },
    };
}


fn cmd_help(_u_input: &mut UserInput) {

    let help = format!( r#"Usage: fswap [COMMAND | help] [OPTIONS...] [ARGUMENTS...]
Note: must be in FSWAP DIR for all commands that do not take it as an argument

COMMANDS
  b[egin]   [SOURCE DIR] [FSWAP DIR]    creates .fswap file linking SOURCE_DIR and FSWAP DIR
  e[nd]     [FSWAP DIR]                 deletes .fswap file, and ALL swapped files
  i[nfo]    [FSWAP DIR]                 prints all swapped files
  r[evert]  [FILES...]                  reverts a swapped file to it's original state (e.g. file.fswap -> file)
  s[wap]    [FILES...]                  if they both exist, swaps a file from SOURCE DIR to FSWAP DIR, and saves the swapped file

OPTIONS
  -a, --all          do command to all files in fswap directory
  -h, --help         print this help
  -r, --recursive    do command to all files in directory
"#);

    println!("{}", help);
}


fn cmd_end(_u_input: &mut UserInput) {
    todo!();
}


fn cmd_swap(u_input: &mut UserInput) {
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

    let sd = str::from_utf8(&buf).unwrap(); // NOTE: failing here shouldn't be my problem
    let source_dir = Path::new(&sd[0..sd_len]);

    if !source_dir.exists() {
        eprintln!("ERROR: '{dir}' does not exist.", dir = source_dir.display());
        exit(1);
    }

    let mut source_file = PathBuf::from(source_dir);

    let files: Vec<String>;
    if !u_input.opts.intersection(Opts::ALL_FILES).is_empty() {
        let swapped_files = match find_fswap_files(&PathBuf::from(".")) {
            Some(x) => x,
            None    => {
                eprintln!("No fswap files found.");
                exit(1);
            },
        };

        files = swapped_files.iter().map(|x| x.strip_suffix(FSWAP_EXT).unwrap().to_string()).collect();
    } else {
        files = u_input.args.clone();
    }

    let mut files = files.iter();
    loop {
        let arg = match files.next() {
            Some(x) => x,
            None    => break,
        };

        let working_file = PathBuf::from(&arg);
        source_file.push(&working_file);

        if !working_file.exists() {
            eprintln!("ERROR: '{file}' doesn't exist.", file = working_file.display());
            exit(1);
        }

        let working_md = working_file.metadata().unwrap_or_else(|err| {
            eprintln!("ERROR: Couldn't get metadata from '{file}': {err}", file = working_file.display());
            exit(1);
        });

        if !working_md.file_type().is_file() {
            eprintln!("ERROR: '{file}' isn't a normal file.", file = working_file.display());
            exit(1);
        }

        if !source_file.exists() {
            eprintln!("ERROR: '{file}' does not exist.", file = source_file.display());
            exit(1);
        }

        let swapped_file = append_to_pathbuf(&working_file, FSWAP_EXT);

        if let Err(err) = fs::rename(&working_file, &swapped_file) {
            eprintln!("ERROR: Couldn't rename '{src}' to '{dest}': {err}", src = working_file.display(), dest = swapped_file.display());
            exit(1);
        }

       if let Err(err) = fs::copy(&source_file, &working_file) {
            eprintln!("ERROR: Couldn't copy '{src}' to '{dest}': {err}", src = source_file.display(), dest = working_file.display());
            exit(1);
       }

        source_file.pop();
    }
}


fn cmd_revert(u_input: &mut UserInput) {
    if let Err(err) = File::open(FSWAP_EXT) {
        eprintln!("ERROR: Couldn't open '{FSWAP_EXT}': {err}");
        exit(1);
    }

    let files: Vec<String>;
    if !u_input.opts.intersection(Opts::ALL_FILES).is_empty() {
        let swapped_files = match find_fswap_files(&PathBuf::from(".")) {
            Some(x) => x,
            None    => {
                eprintln!("No fswap files found.");
                exit(1);
            },
        };

        files = swapped_files.iter().map(|x| x.strip_suffix(FSWAP_EXT).unwrap().to_string()).collect();
    } else {
        files = u_input.args.clone();
    }

    let mut files = files.iter();
    loop {
        let file = match files.next() {
            Some(x) => x,
            None    => break,
        };

        // Misleading name, in this case source_file actually refers to the file that was swapped
        // in FROM the source directory, not a file in the source directory
        let source_file = PathBuf::from(&file);

        if source_file.exists() {
            if let Err(err) = fs::remove_file(&source_file) {
                eprintln!("ERROR: couldn't remove '{file}': {err}", file = source_file.display());
                exit(1);
            }
        }

        let working_file = append_to_pathbuf(&source_file, FSWAP_EXT);
        if !working_file.exists() {
            eprintln!("ERROR: '{file}' doesn't exist.", file = working_file.display());
            exit(1);
        }

        if let Err(err) = fs::rename(&working_file, &source_file) {
            eprintln!("ERROR: Couldn't rename '{work_file}' to '{src_file}': {err}", work_file = working_file.display(), src_file = source_file.display());
            exit(1);
        }
    }
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
        Some(x) => { return Some(PathBuf::from(x)); },
        None    => { return None;                   }
    };
}


// idk if this level of abstractions is even good, but did it 4 learning purposes
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
        let args_out = Self::strip_args_opts(&args_in);
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
        // TODO: print usage based on current command
        println!("Usage: fswap [COMMAND] [OPTIONS...] [ARGUMENTS...]\nSee 'fswap --help' for more information.");
    }


    fn strip_args_opts(args: &Vec<String>) -> Vec<String> {
        // NOTE: '⟡' kinda looks like a face
        return args.clone().into_iter().filter(|x| x.chars().next().unwrap_or('⟡') != '-').collect();
    }


    fn args_to_flags(args: &Vec<String>) -> Opts {
        let mut flags = Opts::NONE;
        for arg in args {
            let mut chars = arg.chars().peekable();
            match chars.next() {
                Some(c) if c == '-' => c,
                _ => continue,
            };

            // peek here bc its valid in case of short opt
            let long_opt = match chars.peek() {
                Some(c) if *c == '-' => true,
                _ => false,
            };

            // can add arg parsing post option here
            if !long_opt {
                loop {
                    match chars.next() {
                        Some(c) if c == 'd' => flags = flags.union(Opts::RECURSIVE),
                        Some(c) if c == 'a' => flags = flags.union(Opts::ALL_FILES),
                        Some(c) if c == 'h' => flags = flags.union(Opts::HELP),
                        Some(_) => {
                            eprintln!("ERROR: Invalid options '{arg}'");
                            exit(1);
                        },
                        None => break,
                    };
                }
            } else {
                match arg.as_str() {
                    "--directory" => flags = flags.union(Opts::RECURSIVE),
                    "--all"       => flags = flags.union(Opts::ALL_FILES),
                    "--help"      => flags = flags.union(Opts::HELP),
                    _ => {
                        eprintln!("ERROR: Invalid options '{arg}'");
                        exit(1);
                    },
                };
            }
        }

        return flags;
    }
}


struct Command {
    name: &'static str,
    short: &'static str,
    func: fn(u_input: &mut UserInput),
}


const NUM_COMMANDS: usize = 6;
const COMMANDS: [Command; NUM_COMMANDS] = [
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


// Commands either succeed or exit the program with non-zero exit code
fn main() {
    let mut u_input = UserInput::new();

    if !u_input.opts.intersection(Opts::HELP).is_empty() {
        cmd_help(&mut u_input);
        exit(0);
    }

    let mut command = String::from("info");
    if u_input.argc > 0 {
        command = u_input.next_arg();
    }

    for cmd in COMMANDS.iter() {
        if cmd.name.eq(&command) {
            (cmd.func)(&mut u_input);
        }

        if cmd.short.eq(&command) {
            (cmd.func)(&mut u_input);
        }
    }

    exit(0);
}
